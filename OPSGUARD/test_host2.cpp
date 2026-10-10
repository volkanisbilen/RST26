// Offline test for the proxy asset hooks: loads <dll> (a d3d9.dll that imports HopeGuard.dll) and, when
// a list is given ("assetFile|plainFile" per line), reads each asset through it and compares.
// Usage: test_host2.exe <full path to d3d9.dll> [list]
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <vector>

int main(int argc, char** argv)
{
    if (argc < 2) return 2;
    HMODULE m = LoadLibraryA(argv[1]);
    if (!m) { printf("LoadLibrary failed %lu\n", GetLastError()); return 2; }
    printf("loaded %s, OPSGUARD %s\n", argv[1], GetModuleHandleA("HopeGuard.dll") ? "present" : "MISSING");
    if (argc < 3) return GetModuleHandleA("HopeGuard.dll") ? 0 : 1;

    typedef int (*ReadFn)(const char*, unsigned char*, int);
    ReadFn rd = (ReadFn)GetProcAddress(m, "ReadLikePanel");
    if (!rd) return 2;
    FILE* l = fopen(argv[2], "r");
    if (!l) return 2;
    char line[1024]; int bad = 0, total = 0;
    std::vector<unsigned char> got(1 << 20);
    while (fgets(line, sizeof(line), l)) {
        line[strcspn(line, "\r\n")] = 0;
        char* bar = strchr(line, '|');
        if (!bar) continue;
        *bar = 0;
        FILE* pf = fopen(bar + 1, "rb");
        fseek(pf, 0, SEEK_END); std::vector<unsigned char> plain(ftell(pf)); fseek(pf, 0, SEEK_SET);
        fread(plain.data(), 1, plain.size(), pf); fclose(pf);
        int n = rd(line, got.data(), (int)got.size());
        bool ok = n == (int)plain.size() && memcmp(got.data(), plain.data(), n) == 0;
        printf("%-12s %6d bytes %s\n", strrchr(line, '\\') + 1, n, ok ? "OK" : "MISMATCH");
        total++; if (!ok) bad++;
    }
    printf("%d files, %d bad\n", total, bad);
    return bad ? 1 : 0;
}
