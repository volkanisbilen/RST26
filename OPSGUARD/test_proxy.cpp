// Offline test stand-in for the d3d9 proxy: built as d3d9.dll, imports HopeGuard.dll like the patched
// proxy does, and reads files the same way the panels do (fopen/fread).
#include <windows.h>
#include <stdio.h>

extern "C" __declspec(dllimport) void HgInit();

// Reads the whole file in the panels' pattern: 4 + 4 + 4 bytes, then the rest in one fread.
extern "C" __declspec(dllexport) int ReadLikePanel(const char* path, unsigned char* out, int cap)
{
    FILE* f = nullptr;
    if (fopen_s(&f, path, "rb") != 0 || !f) return -1;
    int n = (int)fread(out, 1, 4, f);
    n += (int)fread(out + n, 4, 1, f) * 4;
    n += (int)fread(out + n, 4, 1, f) * 4;
    n += (int)fread(out + n, 1, cap - n, f);
    fclose(f);
    return n;
}

BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID)
{
    if (reason == DLL_PROCESS_ATTACH) HgInit();
    return TRUE;
}
