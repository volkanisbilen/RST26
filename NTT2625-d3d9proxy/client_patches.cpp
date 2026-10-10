// Small in-memory code patches for the unpacked 2625 client (exe on disk stays untouched).
// Each patch checks the original bytes first and is skipped if they do not match.
//
//  0x9FDF21  char-select LV label (fn 0x9FDE50, called from 0x7C8866): it printed "%d / %d"
//            (level / rebirth) only when rebirth > 0, else just "%d". NOP the "jle" so the
//            label is always "level / rebirth", e.g. "3 / 0". Found with a SetString trace.
#include <windows.h>
#include <shellapi.h>
#include <psapi.h>
#include <stdio.h>

void Log(const char* msg);

namespace
{
    struct Patch { DWORD addr; BYTE orig[8]; BYTE repl[8]; int len; const char* name; };
    const Patch kPatches[] = {
        { 0x009FDF21, { 0x7E, 0x17 }, { 0x90, 0x90 }, 2, "charselect LV always 'level / rebirth'" },
        // 0xC01EAB  on exit the game calls ShellExecuteA(0, "open", "http://www.nttgame.com", 0, 0, 1):
        //           replace the call with "add esp, 0x18" (same stack cleanup) so no browser opens.
        { 0x00C01EAB, { 0xFF, 0x15, 0xCC, 0x34, 0xF9, 0x00 }, { 0x83, 0xC4, 0x18, 0x90, 0x90, 0x90 }, 6, "no web page on exit" },
        // 0x645B1F  CN3Chr::Load (0x644C00) reads [len][string] into a 512 byte stack buffer; a len >= 512 (old /
        //           incompatible .n3chr, e.g. Chr\mon_2005patos_blue.n3chr) called __report_rangecheckfailure and the
        //           game vanished (fast fail). Instead: "xor al,al; jmp 0x645AFD" = the function's own epilogue, so the
        //           load just fails like a missing file (the caller closes the handle and skips the model).
        { 0x00645B1F, { 0xE8, 0xAA, 0x3F, 0xFA, 0xFF }, { 0x32, 0xC0, 0xEB, 0xDA, 0x90 }, 5, "n3chr load: bad string length fails instead of fast-fail" },
        // 0x930221  the NPC talk balloon (co_pet_balloon.uif) crashed the client; "je +0x31" -> "jmp +0x31" skips it.
        //           Same bytes as in the d3d9.dll that is on the client (its source was not at hand).
        { 0x00930221, { 0x74, 0x31 }, { 0xEB, 0x31 }, 2, "no npc talk balloon (co_pet_balloon.uif crash)" },
        // 0x84B6B4  GM "hold G = run x10" (sets player+0xFFC, speed formula 0x90FAE0) only ran while the GM was invisible
        //           (player+0xBD8, state change type 5): a visible GM (GM panel Visible / Hidden, the panel horse) lost it.
        //           The handler is GM-only already (authority 0 / 0xFA / 0xFB at 0x84B682): "jne skip" -> nops.
        { 0x0084B6B4, { 0x75, 0x09 }, { 0x90, 0x90 }, 2, "GM G-run also while visible / riding" },
        // 0xB214F0  WIZ_GENIE 97 01 07 <id> <on/off> (another player or bot switched the genie): besides the genie mark
        //           over the character it writes "<name> used genie" (text 0x7968) into the chat through 0xA16BB0, and
        //           the farm bots resend that packet every 15 s. "je skip" -> "jmp skip" on both branches: mark stays,
        //           chat line is gone.
        { 0x00B21794, { 0x74, 0x1A }, { 0xEB, 0x1A }, 2, "no 'used genie' chat line (genie on)" },
        { 0x00B218F1, { 0x74, 0x1A }, { 0xEB, 0x1A }, 2, "no 'used genie' chat line (genie off)" },
    };

    // ---------------------------------------------------------------- CN3CPart missing skins
    // CN3CPart::Load (0x65C1F0 from a pack, 0x65B9F0 from a file handle) puts the .n3cskins named in the part into
    // [this+0x124]. With the GPU skin option on ([0x10FD250] == 1) it then builds per-LOD buffers from [this+0x124]
    // without a NULL check -> AV at 0x65C76E "read of 000000B4" when the skins file does not exist. 22 stock item parts
    // point at skins that are in no pack (e.g. 2_0030_01_2 -> item\upc_el_ba_u_upper.n3cskins = El Morad Leather /
    // Novice pants, gloves, shoes; 2_0800_01_3 = Plate / Krowaz helmet), so any unit wearing one crashed the client.
    // The block's "jne skip" is detoured to also skip it when the skins are NULL: the render code already checks
    // [part+0x124], so the part just has no mesh, the same as with the option off.
    DWORD gPartContA = 0x0065C740, gPartSkipA = 0x0065C84D;
    DWORD gPartContB = 0x0065C0A6, gPartSkipB = 0x0065C1B7;

    void __cdecl LogMissingSkins(const BYTE* part, const char* skins)
    {
        static LONG n = 0;
        if (InterlockedIncrement(&n) > 40) return;
        char name[128] = "?";
        __try
        {   // CN3BaseFileAccess file name = std::string at +0x20 (size +0x30, capacity +0x34)
            DWORD len = *(const DWORD*)(part + 0x30), cap = *(const DWORD*)(part + 0x34);
            const char* s = cap > 15 ? *(const char* const*)(part + 0x20) : (const char*)(part + 0x20);
            if (len < sizeof(name)) { memcpy(name, s, len); name[len] = 0; }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        char b[400];
        sprintf_s(b, "CPART: %s -> skins '%.200s' missing, skin buffers skipped (was crash 0x65C76E)", name, skins);
        Log(b);
    }

    // edi = this, [ebp-0x114] = last string read (the skins name), flags = "cmp [0x10FD250], 1"
    __declspec(naked) void PartGuardA()
    {
        __asm {
            jne     skip
            cmp     dword ptr [edi + 0x124], 0
            je      missing
            jmp     dword ptr [gPartContA]
        missing:
            pushad
            lea     eax, [ebp - 0x114]
            push    eax
            push    edi
            call    LogMissingSkins
            add     esp, 8
            popad
        skip:
            jmp     dword ptr [gPartSkipA]
        }
    }

    __declspec(naked) void PartGuardB()
    {
        __asm {
            jne     skip
            cmp     dword ptr [edi + 0x124], 0
            je      missing
            jmp     dword ptr [gPartContB]
        missing:
            pushad
            lea     eax, [ebp - 0x114]
            push    eax
            push    edi
            call    LogMissingSkins
            add     esp, 8
            popad
        skip:
            jmp     dword ptr [gPartSkipB]
        }
    }

    // 6 byte "jne rel32" sites replaced by "jmp guard; nop"
    struct JmpPatch { DWORD addr; BYTE orig[6]; void* target; const char* name; };
    const JmpPatch kJmpPatches[] = {
        { 0x0065C73A, { 0x0F, 0x85, 0x0D, 0x01, 0x00, 0x00 }, (void*)PartGuardA, "n3cpart load (pack): missing skins guard" },
        { 0x0065C0A0, { 0x0F, 0x85, 0x11, 0x01, 0x00, 0x00 }, (void*)PartGuardB, "n3cpart load (file): missing skins guard" },
    };

    void BlockWebPages();
    void InstallBufferCounter();

    // ---------------------------------------------------------------- missing texture guard
    // 0x65C1F0 (a model part's load from the pack, reached through vtable+0x10 from 0x63B2B0) reads a
    // texture name into [ebp-0x114] and asks the texture manager (0x4E4560) for it -> [edi+0x124].
    // A texture missing from the client gives null, and with the option flags [0x118BA54] / [0x1115923]
    // on, 0x65C76E read [null+0xB4] (crash seen in Colony Zone). 0x65C768 "mov eax,[edi+0x124]" -> jmp here:
    // null takes the same exit as "options off" (0x65C73A -> 0x65C84D, after the LOD loop) and the texture name is logged once.
    const DWORD kTexGuardAt = 0x0065C768, kTexGuardOk = 0x0065C76E, kTexGuardSkip = 0x0065C84D;
    void __stdcall LogMissingTexture(const char* name)
    {
        static char seen[64][96]; static int n = 0;
        char nm[96] = "?";
        __try { strncpy_s(nm, name, _TRUNCATE); } __except (EXCEPTION_EXECUTE_HANDLER) {}
        for (int i = 0; i < n; i++) if (!strcmp(seen[i], nm)) return;
        if (n < 64) strcpy_s(seen[n++], nm);
        char b[160]; sprintf_s(b, "TEXGUARD: missing texture '%s' (model part loaded without it)", nm); Log(b);
    }
    __declspec(naked) void TexGuardStub()
    {
        __asm {
            mov  eax, dword ptr [edi + 0x124]
            test eax, eax
            jz   missing
            push kTexGuardOk
            ret
        missing:
            pushad
            lea  eax, [ebp - 0x114]
            push eax
            call LogMissingTexture
            popad
            push kTexGuardSkip
            ret
        }
    }
    void InstallTexGuard()
    {
        BYTE* at = (BYTE*)kTexGuardAt;
        static const BYTE orig[6] = { 0x8B, 0x87, 0x24, 0x01, 0x00, 0x00 };
        if (memcmp(at, orig, 6) != 0) { Log("PATCH: missing texture guard - bytes differ, skipped"); return; }
        DWORD old;
        if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)TexGuardStub - (DWORD)(at + 5); at[5] = 0x90;
        VirtualProtect(at, 6, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 6);
        Log("PATCH: missing texture guard @0065C768 applied");
    }

    // ---------------------------------------------------------------- n3chr part count guard
    // CN3Chr::Load (0x644C00) reads the part count right after the joint name and resizes its part vector with it
    // (0x644E2A push [ebp-0x2A8] / call 0x63FC60). A misread .n3chr (2026-10-02: Chr\npc_mo_shaman2007.n3chr came in
    // 21 bytes shifted, count = 0x68735F6F = the text "o_sh") made the vector throw "vector too long" -> C++ exception,
    // the game closed. Stock files have at most 28 parts, so > 256 means garbage: fail the load through the function's
    // own epilogue (0x645AFD, same exit as the bad string length patch at 0x645B1F) and the caller skips the model.
    const DWORD kPartCntAt = 0x00644E2A, kPartCntOk = 0x00644E30, kChrLoadFail = 0x00645AFD;
    void __stdcall LogBadPartCount(const BYTE* chr, DWORD count)
    {
        static LONG n = 0;
        if (InterlockedIncrement(&n) > 20) return;
        char name[128] = "?";
        __try
        {   // CN3BaseFileAccess file name = std::string at +0x20 (size +0x30, capacity +0x34)
            DWORD len = *(const DWORD*)(chr + 0x30), cap = *(const DWORD*)(chr + 0x34);
            const char* s = cap > 15 ? *(const char* const*)(chr + 0x20) : (const char*)(chr + 0x20);
            if (len < sizeof(name)) { memcpy(name, s, len); name[len] = 0; }
        }
        __except (EXCEPTION_EXECUTE_HANDLER) {}
        char b[260];
        sprintf_s(b, "N3CHR: %s part count %lu (0x%08lX) is garbage, model load failed instead of a crash", name, count, count);
        Log(b);
    }
    // esi = this (CN3Chr), [ebp-0x2A8] = part count just read
    __declspec(naked) void PartCountStub()
    {
        __asm {
            mov  eax, dword ptr [ebp - 0x2A8]
            cmp  eax, 256
            ja   bad
            push eax                        // the replaced "push dword ptr [ebp-0x2A8]"
            push kPartCntOk
            ret
        bad:
            pushad
            push eax
            push esi
            call LogBadPartCount
            popad
            xor  al, al
            push kChrLoadFail
            ret
        }
    }
    void InstallPartCountGuard()
    {
        BYTE* at = (BYTE*)kPartCntAt;
        static const BYTE orig[6] = { 0xFF, 0xB5, 0x58, 0xFD, 0xFF, 0xFF };
        if (memcmp(at, orig, 6) != 0) { Log("PATCH: n3chr part count guard - bytes differ, skipped"); return; }
        DWORD old;
        if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)PartCountStub - (DWORD)(at + 5); at[5] = 0x90;
        VirtualProtect(at, 6, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 6);
        Log("PATCH: n3chr part count guard @00644E2A applied");
    }

    // ---------------------------------------------------------------- chr-type plug (wing) joint matrix guard
    // CN3Chr draw (0x643B40) passes the wing / chr-type plug ([chr+0x204]) the matrix of its joint:
    // [chr+0x1B0] + plug->jointIndex * 64, with no check. After a race change (Fun Class) the joint array can be
    // gone or shorter than the index -> 0x64D670 multiplies an unreadable matrix, AV at 0x4E2CF5. The only call
    // (0x6449CB) goes through this stub: an unreadable matrix skips that plug's draw for the frame.
    DWORD gPlugDraw = 0x0064D670;
    BOOL __stdcall PlugMatrixBad(const void* mtx)
    {
        if ((DWORD_PTR)mtx >= 0x10000 && !IsBadReadPtr(mtx, 64)) return FALSE;
        static LONG n = 0;
        if (InterlockedIncrement(&n) <= 10)
        {
            char b[128]; sprintf_s(b, "PLUGGUARD: joint matrix %p unreadable, wing draw skipped (was crash 0x4E2CF5)", mtx); Log(b);
        }
        return TRUE;
    }
    // ecx = plug, xmm2 = argument, stack: ret, lod, arg, &chr matrix, &joint matrix; callee pops 16
    __declspec(naked) void PlugDrawStub()
    {
        __asm {
            push  ecx
            sub   esp, 4
            movss dword ptr [esp], xmm2
            push  dword ptr [esp + 0x18]        // &joint matrix
            call  PlugMatrixBad
            movss xmm2, dword ptr [esp]
            add   esp, 4
            pop   ecx
            test  eax, eax
            jnz   bad
            jmp   dword ptr [gPlugDraw]
        bad:
            ret   16
        }
    }
    void InstallPlugGuard()
    {
        BYTE* at = (BYTE*)0x006449CB;
        static const BYTE orig[5] = { 0xE8, 0xA0, 0x8C, 0x00, 0x00 };
        if (memcmp(at, orig, 5) != 0) { Log("PATCH: wing joint matrix guard - bytes differ, skipped"); return; }
        DWORD old;
        if (!VirtualProtect(at, 5, PAGE_EXECUTE_READWRITE, &old)) return;
        *(DWORD*)(at + 1) = (DWORD)PlugDrawStub - (DWORD)(at + 5);
        VirtualProtect(at, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 5);
        Log("PATCH: wing joint matrix guard @006449CB applied");
    }

    // ---------------------------------------------------------------- CN3PMesh vertex/index count guard
    // CN3PMesh::Load reads the vertex count and index count from the file and hands them to 0x698C90, which does
    // operator new[] of count * 0x2C (vertices) and count * 2 (indices). A misread count (2026-10-05: a weapon /
    // shield plug Chr\...n3cplug came through the pack read with the count landing on the ascii "item" = 0x6D657469
    // = ~1.8e9) overflows the 0x2C multiply -> operator new throws bad_array_new_length (0x4D953C) and the game
    // closed while loading another player's weapon. The counts arrive at 0x698C90 as [ebp+8] (vertices) and
    // [ebp+0xc] (indices); both only ever come from the two CN3PMesh::Load sites, so a count above any real mesh
    // (cap 0x100000 = 1M verts = 44 MB, far above stock models, far below the garbage) is forced to 0 -> empty
    // mesh, the part just has no model, same harmless result as a missing file.
    const DWORD kPMeshCountAt = 0x00698CB7, kPMeshCont = 0x00698CBD;
    #define PMESH_CAP 0x00100000                 // 1M verts (44 MB) - above any real mesh, below the garbage
    void __stdcall LogBadMeshCount(DWORD vtx, DWORD idx)
    {
        static LONG n = 0;
        if (InterlockedIncrement(&n) > 20) return;
        char b[160];
        sprintf_s(b, "PMESH: vertex/index count %lu/%lu (0x%08lX/0x%08lX) is garbage, mesh emptied instead of a crash",
            vtx, idx, vtx, idx);
        Log(b);
    }
    // edi = this (CN3PMesh), [ebp+8] = vertex count, [ebp+0xc] = index count (both just about to be stored)
    __declspec(naked) void PMeshCountStub()
    {
        __asm {
            mov  eax, dword ptr [ebp + 8]
            cmp  eax, PMESH_CAP
            ja   bad
            mov  eax, dword ptr [ebp + 0x0C]
            cmp  eax, PMESH_CAP
            ja   bad
            mov  eax, dword ptr [ebp + 8]       // the replaced "mov eax,[ebp+8]"
            mov  dword ptr [edi + 0x70], eax     // the replaced "mov [edi+0x70],eax"
            jmp  dword ptr [kPMeshCont]
        bad:
            pushad
            push dword ptr [ebp + 0x0C]
            push dword ptr [ebp + 8]
            call LogBadMeshCount
            popad
            mov  dword ptr [ebp + 8], 0          // clamp both counts so no allocation happens
            mov  dword ptr [ebp + 0x0C], 0
            xor  eax, eax
            mov  dword ptr [edi + 0x70], eax
            jmp  dword ptr [kPMeshCont]           // 0x698CBD stores [ebp+0xc] (=0) into [edi+0x74]
        }
    }
    void InstallPMeshCountGuard()
    {
        BYTE* at = (BYTE*)kPMeshCountAt;
        static const BYTE orig[6] = { 0x8B, 0x45, 0x08, 0x89, 0x47, 0x70 };
        if (memcmp(at, orig, 6) != 0) { Log("PATCH: pmesh count guard - bytes differ, skipped"); return; }
        DWORD old;
        if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)PMeshCountStub - (DWORD)(at + 5); at[5] = 0x90;
        VirtualProtect(at, 6, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 6);
        Log("PATCH: pmesh vertex/index count guard @00698CB7 applied");
    }

    // ---------------------------------------------------------------- player name distance
    // CPlayerOther::Render (0x922E40) draws another player's name / clan / title only while the character is closer
    // than 48.0 to the CAMERA (0x9231C7: movss xmm0,[0x1043CB8]; comiss xmm0,dist). With the camera zoomed out
    // (Camera Range) even players next to us are further than that, so their names disappear. The instruction is
    // pointed at our own limit (the 48.0 constant itself is shared with other code and stays).
    // Option.ini [HopeGuard] NameRange = 48 .. 300 (default 120).
    float g_nameRange = 120.0f;
    void InstallNameRange()
    {
        char ini[MAX_PATH]; GetModuleFileNameA(nullptr, ini, MAX_PATH);
        if (char* s = strrchr(ini, '\\')) strcpy_s(s + 1, MAX_PATH - (s + 1 - ini), "Option.ini");
        int v = (int)GetPrivateProfileIntA("HopeGuard", "NameRange", 120, ini);
        g_nameRange = (float)(v < 48 ? 48 : v > 300 ? 300 : v);

        BYTE* at = (BYTE*)0x009231C7;
        static const BYTE orig[8] = { 0xF3, 0x0F, 0x10, 0x05, 0xB8, 0x3C, 0x04, 0x01 };
        if (memcmp(at, orig, 4) != 0 || (*(DWORD*)(at + 4) != 0x01043CB8 && *(DWORD*)(at + 4) != (DWORD)&g_nameRange))
        { Log("PATCH: player name distance - bytes differ, skipped"); return; }
        DWORD old;
        if (!VirtualProtect(at, 8, PAGE_EXECUTE_READWRITE, &old)) return;
        *(DWORD*)(at + 4) = (DWORD)&g_nameRange;
        VirtualProtect(at, 8, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 8);
        char b[96]; sprintf_s(b, "PATCH: player name distance 48 -> %d @009231C7 applied", (int)g_nameRange); Log(b);
    }

    DWORD WINAPI Apply(LPVOID)
    {
        BlockWebPages();                 // not from DllMain (LoadLibrary under the loader lock)
        // wait until the exe is unpacked (same check as the other hooks: recv prologue or our jmp)
        const BYTE* recv = (const BYTE*)0x0084D0B0;
        static const BYTE kRecvPrologue[5] = { 0x55, 0x8B, 0xEC, 0x6A, 0xFF };
        for (int i = 0; i < 1200 && memcmp(recv, kRecvPrologue, 5) != 0 && recv[0] != 0xE9; i++) Sleep(100);

        for (const Patch& p : kPatches)
        {
            char b[160];
            BYTE* at = (BYTE*)p.addr;
            if (memcmp(at, p.repl, p.len) == 0) { sprintf_s(b, "PATCH: %s already applied", p.name); Log(b); continue; }
            if (memcmp(at, p.orig, p.len) != 0) { sprintf_s(b, "PATCH: %s @%08lX bytes differ, skipped", p.name, p.addr); Log(b); continue; }
            DWORD old;
            if (!VirtualProtect(at, p.len, PAGE_EXECUTE_READWRITE, &old)) continue;
            memcpy(at, p.repl, p.len);
            VirtualProtect(at, p.len, old, &old);
            FlushInstructionCache(GetCurrentProcess(), at, p.len);
            sprintf_s(b, "PATCH: %s @%08lX applied", p.name, p.addr); Log(b);
        }
        InstallTexGuard();
        InstallPartCountGuard();
        InstallPlugGuard();
        InstallPMeshCountGuard();
        InstallBufferCounter();
        InstallNameRange();
        for (const JmpPatch& p : kJmpPatches)
        {
            char b[160];
            BYTE* at = (BYTE*)p.addr;
            BYTE repl[6] = { 0xE9, 0, 0, 0, 0, 0x90 };
            *(DWORD*)(repl + 1) = (DWORD)p.target - (p.addr + 5);
            if (memcmp(at, repl, 6) == 0) { sprintf_s(b, "PATCH: %s already applied", p.name); Log(b); continue; }
            if (memcmp(at, p.orig, 6) != 0) { sprintf_s(b, "PATCH: %s @%08lX bytes differ, skipped", p.name, p.addr); Log(b); continue; }
            DWORD old;
            if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) continue;
            memcpy(at, repl, 6);
            VirtualProtect(at, 6, old, &old);
            FlushInstructionCache(GetCurrentProcess(), at, 6);
            sprintf_s(b, "PATCH: %s @%08lX applied", p.name, p.addr); Log(b);
        }
        return 0;
    }

    // ---------------------------------------------------------------- no web pages
    // Any http(s) page the game (or a DLL in it) tries to open through ShellExecute is dropped and logged.
    // Detour on the API entry (mov edi,edi / push ebp / mov ebp,esp = 5 position independent bytes).
    typedef HINSTANCE(WINAPI* tSEA)(HWND, LPCSTR, LPCSTR, LPCSTR, LPCSTR, INT);
    typedef HINSTANCE(WINAPI* tSEW)(HWND, LPCWSTR, LPCWSTR, LPCWSTR, LPCWSTR, INT);
    typedef BOOL(WINAPI* tSEEA)(SHELLEXECUTEINFOA*);
    typedef BOOL(WINAPI* tSEEW)(SHELLEXECUTEINFOW*);
    tSEA oSEA; tSEW oSEW; tSEEA oSEEA; tSEEW oSEEW;

    bool IsWebA(LPCSTR s) { return s && (_strnicmp(s, "http:", 5) == 0 || _strnicmp(s, "https:", 6) == 0 || _strnicmp(s, "www.", 4) == 0); }
    bool IsWebW(LPCWSTR s) { return s && (_wcsnicmp(s, L"http:", 5) == 0 || _wcsnicmp(s, L"https:", 6) == 0 || _wcsnicmp(s, L"www.", 4) == 0); }
    void LogBlockedA(LPCSTR s) { char b[300]; sprintf_s(b, "WEB: blocked %.250s", s); Log(b); }
    void LogBlockedW(LPCWSTR s) { char b[300]; sprintf_s(b, "WEB: blocked %.250ls", s); Log(b); }

    HINSTANCE WINAPI hkSEA(HWND h, LPCSTR op, LPCSTR file, LPCSTR par, LPCSTR dir, INT show)
    {
        if (IsWebA(file) || IsWebA(par)) { LogBlockedA(IsWebA(file) ? file : par); return (HINSTANCE)33; }
        return oSEA(h, op, file, par, dir, show);
    }
    HINSTANCE WINAPI hkSEW(HWND h, LPCWSTR op, LPCWSTR file, LPCWSTR par, LPCWSTR dir, INT show)
    {
        if (IsWebW(file) || IsWebW(par)) { LogBlockedW(IsWebW(file) ? file : par); return (HINSTANCE)33; }
        return oSEW(h, op, file, par, dir, show);
    }
    BOOL WINAPI hkSEEA(SHELLEXECUTEINFOA* i)
    {
        if (i && (IsWebA(i->lpFile) || IsWebA(i->lpParameters))) { LogBlockedA(IsWebA(i->lpFile) ? i->lpFile : i->lpParameters); i->hInstApp = (HINSTANCE)33; return TRUE; }
        return oSEEA(i);
    }
    BOOL WINAPI hkSEEW(SHELLEXECUTEINFOW* i)
    {
        if (i && (IsWebW(i->lpFile) || IsWebW(i->lpParameters))) { LogBlockedW(IsWebW(i->lpFile) ? i->lpFile : i->lpParameters); i->hInstApp = (HINSTANCE)33; return TRUE; }
        return oSEEW(i);
    }

    void* HookApi(HMODULE mod, const char* name, void* hook)
    {
        BYTE* p = (BYTE*)GetProcAddress(mod, name);
        static const BYTE kHot[5] = { 0x8B, 0xFF, 0x55, 0x8B, 0xEC };
        if (!p || memcmp(p, kHot, 5) != 0) { char b[96]; sprintf_s(b, "WEB: %s not hooked", name); Log(b); return nullptr; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return nullptr;
        memcpy(tramp, p, 5);
        tramp[5] = 0xE9; *(DWORD*)(tramp + 6) = (DWORD)(p + 5) - (DWORD)(tramp + 10);
        DWORD old;
        VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old);
        p[0] = 0xE9; *(DWORD*)(p + 1) = (DWORD)hook - (DWORD)(p + 5);
        VirtualProtect(p, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), p, 5);
        return tramp;
    }

    void BlockWebPages()
    {
        HMODULE sh = LoadLibraryA("shell32.dll");
        if (!sh) return;
        oSEA = (tSEA)HookApi(sh, "ShellExecuteA", (void*)hkSEA);
        oSEW = (tSEW)HookApi(sh, "ShellExecuteW", (void*)hkSEW);
        oSEEA = (tSEEA)HookApi(sh, "ShellExecuteExA", (void*)hkSEEA);
        oSEEW = (tSEEW)HookApi(sh, "ShellExecuteExW", (void*)hkSEEW);
        Log("WEB: shell execute hooks installed");
    }
}

namespace
{
    // ---------------------------------------------------------------- memory monitor
    // 2026-10-01 crashes with std::bad_alloc (0x5E64EF via 0x6D26EA / 0x4D953C) were the 32-bit address space
    // running out (~1.8 GB commit at the crash) with many bots in view. Logs commit / working set / used and
    // largest free address range every 30 s (and 10 s once it gets tight) so the next one shows the trend.
    // DX11 buffer statistics (2026-10-04, "why does DX11 run out of memory"): every D3D11 buffer the game creates goes
    // through 0x8B32B0 (fastcall: ecx = holder, edx = initial data, stack: count, stride, usage; usage 2 = dynamic).
    // The model part loader (return address 0x65C84A pack / 0x65C1B4 file) makes one dynamic buffer per part instance.
    // Counted here (creations, not live objects) and logged with the address space map by MemMonitor.
    volatile LONG gBufDynN = 0, gBufDynKB = 0, gBufPartN = 0, gBufPartKB = 0, gBufStatN = 0, gBufStatKB = 0;
    void* gBufTramp = nullptr;
    void __stdcall CountBuffer(DWORD ret, DWORD count, DWORD stride, DWORD usage)
    {
        LONG kb = (LONG)(((unsigned long long)count * stride + 1023) >> 10);
        if (usage == 2)
        {
            InterlockedIncrement(&gBufDynN); InterlockedExchangeAdd(&gBufDynKB, kb);
            if (ret == 0x0065C84A || ret == 0x0065C1B4) { InterlockedIncrement(&gBufPartN); InterlockedExchangeAdd(&gBufPartKB, kb); }
        }
        else { InterlockedIncrement(&gBufStatN); InterlockedExchangeAdd(&gBufStatKB, kb); }
    }
    __declspec(naked) void BufferCountStub()
    {
        __asm {
            pushfd
            pushad
            push dword ptr [esp + 0x30]         // usage
            push dword ptr [esp + 0x30]         // stride
            push dword ptr [esp + 0x30]         // count
            push dword ptr [esp + 0x30]         // return address
            call CountBuffer
            popad
            popfd
            jmp  dword ptr [gBufTramp]
        }
    }
    void InstallBufferCounter()
    {
        BYTE* at = (BYTE*)0x008B32B0;
        static const BYTE orig[6] = { 0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8 };   // push ebp / mov ebp,esp / and esp,-8
        if (memcmp(at, orig, 6) != 0) { Log("MEM: buffer counter - bytes differ, skipped"); return; }
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return;
        memcpy(tramp, at, 6);
        tramp[6] = 0xE9; *(DWORD*)(tramp + 7) = (DWORD)(at + 6) - (DWORD)(tramp + 11);
        gBufTramp = tramp;
        DWORD old;
        if (!VirtualProtect(at, 6, PAGE_EXECUTE_READWRITE, &old)) return;
        at[0] = 0xE9; *(DWORD*)(at + 1) = (DWORD)BufferCountStub - (DWORD)(at + 5); at[5] = 0x90;
        VirtualProtect(at, 6, old, &old);
        FlushInstructionCache(GetCurrentProcess(), at, 6);
        Log("MEM: D3D11 buffer counter installed @008B32B0");
    }

    DWORD WINAPI MemMonitor(LPVOID)
    {
        typedef BOOL(WINAPI* tGPMI)(HANDLE, PPROCESS_MEMORY_COUNTERS, DWORD);
        tGPMI pGPMI = (tGPMI)GetProcAddress(GetModuleHandleA("kernel32.dll"), "K32GetProcessMemoryInfo");
        DWORD lastLog = 0;
        for (;;)
        {
            Sleep(5000);
            SIZE_T used = 0, largestFree = 0;
            // address space by allocation: private ones in size classes (count / KB), mapped, image
            unsigned cN[4] = {}, cKB[4] = {}, mapKB = 0, imgKB = 0; PVOID curBase = nullptr; SIZE_T curSize = 0; DWORD curType = 0;
            auto flush = [&]() {
                if (!curSize) return;
                unsigned kb = (unsigned)(curSize >> 10);
                if (curType == MEM_IMAGE) imgKB += kb; else if (curType == MEM_MAPPED) mapKB += kb;
                else { int c = curSize <= 0x10000 ? 0 : curSize <= 0x100000 ? 1 : curSize <= 0x1000000 ? 2 : 3; cN[c]++; cKB[c] += kb; }
                curSize = 0; };
            MEMORY_BASIC_INFORMATION mbi;
            for (DWORD_PTR p = 0x10000; VirtualQuery((LPCVOID)p, &mbi, sizeof(mbi)) == sizeof(mbi); )
            {
                if (mbi.State == MEM_FREE) { flush(); curBase = nullptr; if (mbi.RegionSize > largestFree) largestFree = mbi.RegionSize; }
                else
                {
                    used += mbi.RegionSize;
                    if (mbi.AllocationBase != curBase) { flush(); curBase = mbi.AllocationBase; curType = mbi.Type; }
                    curSize += mbi.RegionSize;
                }
                DWORD_PTR next = (DWORD_PTR)mbi.BaseAddress + mbi.RegionSize;
                if (next <= p) break;       // wrapped past the top of the address space
                p = next;
            }
            bool tight = largestFree < 128u * 1024 * 1024 || used > 3200u * 1024 * 1024;
            DWORD now = GetTickCount();
            if (lastLog && now - lastLog < (tight ? 10000u : 30000u)) continue;
            lastLog = now;
            PROCESS_MEMORY_COUNTERS_EX pmc = {};
            pmc.cb = sizeof(pmc);
            if (pGPMI) pGPMI(GetCurrentProcess(), (PPROCESS_MEMORY_COUNTERS)&pmc, sizeof(pmc));
            char b[420];
            sprintf_s(b, "MEM: commit=%uMB ws=%uMB va_used=%uMB largest_free=%uMB%s", (unsigned)(pmc.PrivateUsage >> 20),
                (unsigned)(pmc.WorkingSetSize >> 20), (unsigned)(used >> 20), (unsigned)(largestFree >> 20), tight ? "  LOW!" : "");
            Log(b);
            flush();
            sprintf_s(b, "MEMMAP: priv<=64K %u/%uMB <=1M %u/%uMB <=16M %u/%uMB >16M %u/%uMB mapped %uMB image %uMB | D3D11 buffers made: part %ld/%ldMB other dyn %ld/%ldMB static %ld/%ldMB",
                cN[0], cKB[0] >> 10, cN[1], cKB[1] >> 10, cN[2], cKB[2] >> 10, cN[3], cKB[3] >> 10, mapKB >> 10, imgKB >> 10,
                gBufPartN, gBufPartKB >> 10, gBufDynN - gBufPartN, (gBufDynKB - gBufPartKB) >> 10, gBufStatN, gBufStatKB >> 10);
            Log(b);
        }
    }
}

void ClientPatches_Init()
{
    CloseHandle(CreateThread(nullptr, 0, Apply, nullptr, 0, nullptr));
    CloseHandle(CreateThread(nullptr, 0, MemMonitor, nullptr, 0, nullptr));
}

// Other players' names: CGameProcMain keeps a "draw the other players' names" switch at +0x85C (2140). InitZone
// (0x81B170) turns it on, the WIZ_EVENT (0x5F) sub 1 handler (0x7FEA50, event start + "Time left" timer) turns it
// off and nothing turns it back on before the next zone change: every other player then walks around without a
// name. Game thread (ProxyWndProc), twice a second: put it back and log the first few times it was found off.
void NameFlag_Tick()
{
    static DWORD s_last = 0; static int s_logged = 0;
    DWORD now = GetTickCount();
    if (now - s_last < 500) return;
    s_last = now;
    __try
    {
        BYTE* proc = *(BYTE**)0x011158FC;
        BYTE* me = *(BYTE**)0x01115834;
        if (!proc || !me) return;
        if (proc[0x85C] == 0)
        {
            proc[0x85C] = 1;
            if (s_logged < 30) { s_logged++; Log("NAMES: 'draw player names' switch was OFF -> turned back on"); }
        }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) {}
}