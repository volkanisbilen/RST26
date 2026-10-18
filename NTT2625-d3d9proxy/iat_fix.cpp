// Fixes IAT slots the unpacked KnightOnLine.exe dump left pointing at packer stubs.
// Those stubs jump to API addresses from the session the dump was taken in (ASLR changes them
// every boot) -> "execute at 768C4380" access violation.
//
//  0xF935A8  USER32!wsprintfA  - only reference: 0x8F3E78 (cloak model path "Item\Cloak_%.3d.n3cplug",
//                                crashed whenever a caped character was loaded). Found with crash_trap.
#include <windows.h>
#include <stdio.h>

void Log(const char* msg);

namespace
{
    struct Slot { DWORD addr; const char* dll; const char* name; };
    const Slot kSlots[] = {
        { 0x00F935A8, "user32.dll", "wsprintfA" },
    };

    // packer stub section of the unpack build; a slot pointing here is stale
    bool InStubSection(DWORD v, DWORD& lo, DWORD& hi)
    {
        BYTE* base = (BYTE*)GetModuleHandleA(nullptr);
        auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
        auto sec = IMAGE_FIRST_SECTION(nt);
        for (WORD i = 0; i < nt->FileHeader.NumberOfSections; i++, sec++)
        {
            if (memcmp(sec->Name, ".kgap0", 6) == 0)
            {
                lo = (DWORD)base + sec->VirtualAddress;
                hi = lo + sec->Misc.VirtualSize;
                return v >= lo && v < hi;
            }
        }
        return false;
    }
}

void IatFix_Init()
{
    for (const Slot& s : kSlots)
    {
        char b[160];
        DWORD* slot = (DWORD*)s.addr;
        MEMORY_BASIC_INFORMATION mbi{};
        if (!VirtualQuery(slot, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT)
        {
            sprintf_s(b, "IATFIX: %s slot %08lX not mapped, skipped", s.name, s.addr); Log(b);
            continue;
        }
        DWORD lo = 0, hi = 0, cur = *slot;
        if (!InStubSection(cur, lo, hi))
        {
            sprintf_s(b, "IATFIX: %s slot %08lX = %08lX (not a stub), left alone", s.name, s.addr, cur); Log(b);
            continue;
        }
        FARPROC fn = GetProcAddress(GetModuleHandleA(s.dll), s.name);   // user32 is already loaded (DllMain: no LoadLibrary)
        DWORD old;
        if (!fn || !VirtualProtect(slot, 4, PAGE_READWRITE, &old))
        {
            sprintf_s(b, "IATFIX: %s could not be fixed (fn=%p err=%lu)", s.name, fn, GetLastError()); Log(b);
            continue;
        }
        *slot = (DWORD)fn;
        VirtualProtect(slot, 4, old, &old);
        sprintf_s(b, "IATFIX: %s slot %08lX %08lX -> %08lX", s.name, s.addr, cur, (DWORD)fn); Log(b);
    }
}
