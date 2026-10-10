// Offline test for HopeGuard.dll: this exe contains a copy of the game's pack read site (same bytes),
// loads the DLL, then reads the entries of a fake pack through it and compares with the plain files.
// Usage: test_host.exe <src file> <manifest: "start size plainfile" per line>
#include <windows.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <vector>

struct PakHandle { uint32_t pack, pos, size, start; };

static std::vector<uint8_t> g_src;
static std::vector<uint8_t> g_buf;      // like the game's buffer mode: every Read overwrites it

struct MemFile {
    virtual ~MemFile() {}
    virtual int Size() { return (int)g_src.size(); }
    virtual const uint8_t* Read(uint32_t off, uint32_t n)
    {
        if (off >= g_src.size() || off + n > g_src.size()) return nullptr;
        g_buf.assign(g_src.begin() + off, g_src.begin() + off + n);
        return g_buf.data();
    }
};

static void* __cdecl Move(void* d, const void* s, size_t n) { return memmove(d, s, n); }

__declspec(naked) static int __stdcall FakePakRead(void* pack, void* dst, uint32_t n, PakHandle* h)
{
    __asm {
        push ebp
        mov ebp, esp
        push esi
        mov esi, [ebp + 0x14]
        mov edx, [ebp + 0x10]
        mov ecx, [ebp + 8]
        push edx
        push dword ptr [esi + 4]
        mov ecx, [ecx + 0x24]
        mov eax, [ecx]
        call dword ptr [eax + 8]
        test eax, eax
        je fail
        push dword ptr [ebp + 0x10]
        push eax
        push dword ptr [ebp + 0x0C]
        call Move
        mov eax, [ebp + 0x10]
        add esp, 0x0C
        add [esi + 4], eax
        mov eax, 1
        jmp done
    fail:
        xor eax, eax
    done:
        pop esi
        pop ebp
        ret 16
    }
}

int main(int argc, char** argv)
{
    if (argc < 3) return 2;
    FILE* f = fopen(argv[1], "rb");
    if (!f) return 2;
    fseek(f, 0, SEEK_END); g_src.resize(ftell(f)); fseek(f, 0, SEEK_SET);
    fread(g_src.data(), 1, g_src.size(), f); fclose(f);

    if (!LoadLibraryA("HopeGuard.dll")) { printf("LoadLibrary failed %lu\n", GetLastError()); return 2; }

    uint8_t packObj[0x28] = {};
    MemFile mf;
    *(void**)(packObj + 0x24) = &mf;

    FILE* m = fopen(argv[2], "r");
    if (!m) return 2;
    unsigned start, size; char name[260]; int bad = 0, total = 0;
    srand(1234);
    while (fscanf(m, "%u %u %259s", &start, &size, name) == 3) {
        FILE* pf = fopen(name, "rb");
        fseek(pf, 0, SEEK_END); std::vector<uint8_t> plain(ftell(pf)); fseek(pf, 0, SEEK_SET);
        fread(plain.data(), 1, plain.size(), pf); fclose(pf);

        // two entries read interleaved would thrash a 1-slot cache; random chunk sizes cover unaligned offsets
        PakHandle h = { 0, start, size, start };
        std::vector<uint8_t> got(plain.size());
        size_t done = 0; bool ok = true;
        while (done < plain.size()) {
            uint32_t n = 1 + rand() % 37;
            if (n > plain.size() - done) n = (uint32_t)(plain.size() - done);
            if (!FakePakRead(packObj, got.data() + done, n, &h)) { ok = false; break; }
            done += n;
        }
        ok = ok && h.pos == start + plain.size() && got == plain;
        printf("%-24s start %8u size %6u %s\n", name, start, size, ok ? "OK" : "MISMATCH");
        total++; if (!ok) bad++;
    }
    fclose(m);
    printf("%d entries, %d bad\n", total, bad);
    return bad ? 1 : 0;
}
