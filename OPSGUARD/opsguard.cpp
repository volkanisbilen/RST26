// HopeGuard.dll (formerly OPSGUARD.dll): decrypts HopeGuard-encrypted pack entries (UI\ui.src *.uif) while the game reads them.
// The d3d9 proxy loads this DLL from the game folder when it exists.
//
// Every pack read in the 2625 client goes through one function (0x00631CC0 in the unpacked exe):
//   PakRead(handle, dst, n, ...) { p = pack->memFile->Read(handle->pos, n); memmove(dst, p, n); handle->pos += n; }
// We redirect that memmove call. The site is found by signature, so other builds of the exe work too.
#include <windows.h>
#include <stdio.h>
#include <share.h>
#include <stdarg.h>
#include <string.h>
#include "hgcrypt.h"

static void Log(const char* fmt, ...)
{
    char path[MAX_PATH];
    GetModuleFileNameA(nullptr, path, MAX_PATH);
    char* slash = strrchr(path, '\\');
    if (!slash) return;
    strcpy_s(slash + 1, MAX_PATH - (slash + 1 - path), "hopeguard.log");
    FILE* f = _fsopen(path, "a", _SH_DENYNO);
    if (!f) return;
    SYSTEMTIME t; GetLocalTime(&t);
    fprintf(f, "[%02d:%02d:%02d] pid=%lu ", t.wHour, t.wMinute, t.wSecond, GetCurrentProcessId());
    va_list a; va_start(a, fmt); vfprintf(f, fmt, a); va_end(a);
    fputc('\n', f);
    fclose(f);
}

struct PakHandle { uint32_t pack, pos, size, start; };     // the game's 16-byte virtual file handle

// CMyMemFile::Read(offset, n): thiscall, returns a pointer to the bytes (mapped view or its own buffer)
typedef const uint8_t* (__fastcall* PakReadFn)(void* self, void* edx, uint32_t off, uint32_t n);

struct CacheSlot { void* pack; uint32_t start, size, seed; bool enc; };
static CacheSlot g_cache[64];       // only touched under the game's pack lock (held around PakRead)

static const CacheSlot* Lookup(void* packObj, uint32_t start, uint32_t size)
{
    CacheSlot* s = &g_cache[((start >> 4) ^ size) & 63];
    if (s->pack == packObj && s->start == start && s->size == size)
        return s;
    s->pack = packObj; s->start = start; s->size = size; s->enc = false;
    void* mf = *(void**)((uint8_t*)packObj + 0x24);
    if (mf && size > hg::kTrailer) {
        PakReadFn rd = (*(PakReadFn**)mf)[2];
        const uint8_t* t = rd(mf, nullptr, start + size - hg::kTrailer, hg::kTrailer);
        if (t) {
            uint32_t nonce, tag;
            memcpy(&nonce, t, 4); memcpy(&tag, t + 4, 4);
            if (tag == hg::Tag(nonce, size)) { s->enc = true; s->seed = hg::Seed(nonce, size); }
        }
    }
    return s;
}

static void* __cdecl PakCopy(uint8_t* dst, const uint8_t* src, size_t n, PakHandle* h, void* packObj)
{
    memmove(dst, src, n);           // first: the trailer probe below may reuse the buffer src points into
    if (n && h && packObj) {
        const CacheSlot* s = Lookup(packObj, h->start, h->size);
        if (s->enc)
            hg::Crypt(dst, (uint32_t)n, h->pos - h->start, s->seed);
    }
    return dst;
}

// Replaces `call memmove` in PakRead: esi = handle, [ebp+8] = pack object, cdecl args dst/src/n on the stack.
__declspec(naked) static void PakCopyStub()
{
    __asm {
        push dword ptr [ebp + 8]
        push esi
        push dword ptr [esp + 0x14]     // n
        push dword ptr [esp + 0x14]     // src
        push dword ptr [esp + 0x14]     // dst
        call PakCopy
        add esp, 20
        ret
    }
}

// mov ecx,[ebp+8] / push edx / push [esi+4] / mov ecx,[ecx+24h] / mov eax,[ecx] / call [eax+8] / test eax,eax /
// je .. / push [ebp+10h] / push eax / push [ebp+0Ch] / call memmove / mov eax,[ebp+10h] / add esp,0Ch / add [esi+4],eax
static const short kSig[] = {
    0x8B, 0x4D, 0x08, 0x52, 0xFF, 0x76, 0x04, 0x8B, 0x49, 0x24, 0x8B, 0x01, 0xFF, 0x50, 0x08, 0x85, 0xC0,
    0x74, -1, 0xFF, 0x75, 0x10, 0x50, 0xFF, 0x75, 0x0C, 0xE8, -1, -1, -1, -1,
    0x8B, 0x45, 0x10, 0x83, 0xC4, 0x0C, 0x01, 0x46, 0x04
};
static const int kSigLen = sizeof(kSig) / sizeof(kSig[0]), kCallOfs = 26;

static bool Match(const uint8_t* p)
{
    for (int i = 0; i < kSigLen; i++)
        if (kSig[i] >= 0 && p[i] != (uint8_t)kSig[i]) return false;
    return true;
}

static int FindSites(uint8_t** out, int maxOut)
{
    uint8_t* base = (uint8_t*)GetModuleHandleA(nullptr);
    IMAGE_NT_HEADERS* nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    uint8_t* end = base + nt->OptionalHeader.SizeOfImage;
    int found = 0;
    MEMORY_BASIC_INFORMATION mbi;
    for (uint8_t* p = base; p < end && VirtualQuery(p, &mbi, sizeof(mbi)); p = (uint8_t*)mbi.BaseAddress + mbi.RegionSize) {
        if (mbi.State != MEM_COMMIT || (mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)))
            continue;
        // only code: PakRead lives in an executable page
        if (!(mbi.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)))
            continue;
        uint8_t* s = (uint8_t*)mbi.BaseAddress;
        uint8_t* e = s + mbi.RegionSize;
        if (e > end) e = end;
        for (uint8_t* q = s; q + kSigLen <= e; q++)
            if (*q == 0x8B && Match(q)) {
                if (found < maxOut) out[found] = q;
                found++;
            }
    }
    return found;
}

static void Install()
{
    uint8_t* sites[4];
    int n = FindSites(sites, 4);
    if (n != 1) {
        Log("PAK: read site signature matched %d time(s), expected 1 -> NOT hooked, encrypted UI files will not load", n);
        return;
    }
    uint8_t* call = sites[0] + kCallOfs;
    int32_t rel = (int32_t)((uint8_t*)PakCopyStub - (call + 5));
    DWORD old;
    if (!VirtualProtect(call, 5, PAGE_EXECUTE_READWRITE, &old)) {
        Log("PAK: VirtualProtect failed (err=%lu) at %p -> NOT hooked", GetLastError(), call);
        return;
    }
    memcpy(call + 1, &rel, 4);
    VirtualProtect(call, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), call, 5);
    Log("PAK: decrypt hook installed at %p", call);
}

// ---- proxy assets: HopeGuard\*_ui\*.pus -------------------------------------------------------------
// The d3d9 proxy reads them with fopen/fread, i.e. CreateFileW + ReadFile through its own import table.
// Those imports are redirected here, so encrypted files are decrypted as they are read.

struct AssetHandle { HANDLE h; uint32_t seed; };
static AssetHandle g_assets[64];
static SRWLOCK g_assetLock = SRWLOCK_INIT;
static volatile LONG g_assetCount = 0;

static void AssetForget(HANDLE h)
{
    if (!g_assetCount) return;
    AcquireSRWLockExclusive(&g_assetLock);
    for (AssetHandle& a : g_assets)
        if (a.h == h) { a.h = nullptr; InterlockedDecrement(&g_assetCount); }
    ReleaseSRWLockExclusive(&g_assetLock);
}

// name: "...\HopeGuard\...\file.pus" -> lower-case file name, or false
static bool AssetName(const wchar_t* path, char* out, size_t cap)
{
    size_t n = wcslen(path);
    if (n < 5 || (_wcsicmp(path + n - 4, L".pus") != 0 && _wcsicmp(path + n - 4, L".txt") != 0)) return false;
    bool inDir = false;
    for (const wchar_t* p = path; *p; p++)
        if ((*p == L'\\' || *p == L'/') && _wcsnicmp(p + 1, L"HopeGuard", 9) == 0 && (p[10] == L'\\' || p[10] == L'/')) { inDir = true; break; }
    if (!inDir) return false;
    const wchar_t* base = path + n;
    while (base > path && base[-1] != L'\\' && base[-1] != L'/') base--;
    size_t i = 0;
    for (; base[i] && i + 1 < cap; i++) {
        wchar_t c = base[i];
        if (c > 0x7F) return false;
        out[i] = (char)((c >= 'A' && c <= 'Z') ? c + 32 : c);
    }
    out[i] = 0;
    return base[i] == 0;
}

static bool IsText(const uint8_t* p)
{
    for (int i = 0; i < 16; i++)
        if (!((p[i] >= 0x20 && p[i] < 0x7F) || p[i] == '\r' || p[i] == '\n' || p[i] == '\t')) return false;
    return true;
}

static void AssetProbe(HANDLE h, const wchar_t* path)
{
    char name[128];
    if (!AssetName(path, name, sizeof(name))) return;
    LARGE_INTEGER size, zero = {};
    uint8_t head[16]; DWORD got = 0;
    if (!GetFileSizeEx(h, &size) || size.QuadPart < 16 || size.HighPart) return;
    BOOL ok = ReadFile(h, head, 16, &got, nullptr);
    SetFilePointerEx(h, zero, nullptr, FILE_BEGIN);
    if (!ok || got != 16) return;
    // .pus: "PUSI" magic; .txt: the first 16 bytes are printable text. Plain files pass through.
    bool text = name[strlen(name) - 1] == 't';
    if (text ? IsText(head) : memcmp(head, "PUSI", 4) == 0) return;
    uint32_t seed = hg::AssetSeed(size.LowPart, hg::NameHash(name));
    hg::Crypt(head, 16, 0, seed);
    if (text ? !IsText(head) : memcmp(head, "PUSI", 4) != 0) return;  // neither plain nor ours: leave as is
    AcquireSRWLockExclusive(&g_assetLock);
    for (AssetHandle& a : g_assets)
        if (!a.h) { a.h = h; a.seed = seed; InterlockedIncrement(&g_assetCount); break; }
    ReleaseSRWLockExclusive(&g_assetLock);
}

static HANDLE WINAPI HkCreateFileW(LPCWSTR name, DWORD access, DWORD share, LPSECURITY_ATTRIBUTES sa, DWORD disp, DWORD flags, HANDLE tmpl)
{
    HANDLE h = CreateFileW(name, access, share, sa, disp, flags, tmpl);
    if (h != INVALID_HANDLE_VALUE) {
        AssetForget(h);             // handle value reused after a close we did not see
        if (name && disp == OPEN_EXISTING && (access & GENERIC_READ) && !(access & GENERIC_WRITE)) {
            DWORD err = GetLastError();
            AssetProbe(h, name);
            SetLastError(err);
        }
    }
    return h;
}

static HANDLE WINAPI HkCreateFileA(LPCSTR name, DWORD access, DWORD share, LPSECURITY_ATTRIBUTES sa, DWORD disp, DWORD flags, HANDLE tmpl)
{
    wchar_t w[MAX_PATH * 2];
    if (!name || !MultiByteToWideChar(CP_ACP, 0, name, -1, w, MAX_PATH * 2))
        return CreateFileA(name, access, share, sa, disp, flags, tmpl);
    return HkCreateFileW(w, access, share, sa, disp, flags, tmpl);
}

static BOOL WINAPI HkReadFile(HANDLE h, LPVOID buf, DWORD n, LPDWORD pRead, LPOVERLAPPED ov)
{
    uint32_t seed = 0; bool ours = false;
    if (g_assetCount && !ov) {
        AcquireSRWLockShared(&g_assetLock);
        for (const AssetHandle& a : g_assets)
            if (a.h == h) { seed = a.seed; ours = true; break; }
        ReleaseSRWLockShared(&g_assetLock);
    }
    if (!ours)
        return ReadFile(h, buf, n, pRead, ov);
    LARGE_INTEGER zero = {}, pos = {};
    SetFilePointerEx(h, zero, &pos, FILE_CURRENT);
    DWORD got = 0;
    BOOL ok = ReadFile(h, buf, n, &got, nullptr);
    if (ok && got)
        hg::Crypt((uint8_t*)buf, got, pos.LowPart, seed);
    if (pRead) *pRead = got;
    return ok;
}

static BOOL WINAPI HkCloseHandle(HANDLE h)
{
    AssetForget(h);
    return CloseHandle(h);
}

// Redirects KERNEL32 imports of one loaded module. Returns how many slots were changed.
static int PatchImports(HMODULE mod)
{
    static const struct { const char* name; void* hook; } kHooks[] = {
        { "CreateFileW", HkCreateFileW }, { "CreateFileA", HkCreateFileA },
        { "ReadFile", HkReadFile }, { "CloseHandle", HkCloseHandle },
    };
    uint8_t* base = (uint8_t*)mod;
    IMAGE_NT_HEADERS* nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    DWORD rva = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress;
    if (!rva) return 0;
    int done = 0;
    for (IMAGE_IMPORT_DESCRIPTOR* d = (IMAGE_IMPORT_DESCRIPTOR*)(base + rva); d->Name; d++) {
        if (_stricmp((const char*)base + d->Name, "KERNEL32.dll") != 0 || !d->OriginalFirstThunk) continue;
        IMAGE_THUNK_DATA* names = (IMAGE_THUNK_DATA*)(base + d->OriginalFirstThunk);
        IMAGE_THUNK_DATA* slots = (IMAGE_THUNK_DATA*)(base + d->FirstThunk);
        for (; names->u1.AddressOfData; names++, slots++) {
            if (IMAGE_SNAP_BY_ORDINAL(names->u1.Ordinal)) continue;
            const char* fn = ((IMAGE_IMPORT_BY_NAME*)(base + names->u1.AddressOfData))->Name;
            for (const auto& k : kHooks) {
                if (strcmp(fn, k.name) != 0) continue;
                DWORD old;
                if (VirtualProtect(&slots->u1.Function, sizeof(void*), PAGE_READWRITE, &old)) {
                    slots->u1.Function = (ULONG_PTR)k.hook;
                    VirtualProtect(&slots->u1.Function, sizeof(void*), old, &old);
                    done++;
                }
            }
        }
    }
    return done;
}

static void InstallAssets()
{
    HMODULE proxy = GetModuleHandleA("d3d9.dll");
    if (!proxy) { Log("ASSET: d3d9.dll not loaded -> proxy assets not hooked"); return; }
    char path[MAX_PATH] = {};
    GetModuleFileNameA(proxy, path, MAX_PATH);
    char sys[MAX_PATH] = {};
    GetSystemDirectoryA(sys, MAX_PATH);
    if (_strnicmp(path, sys, strlen(sys)) == 0) { Log("ASSET: d3d9.dll is the system one, nothing to hook"); return; }
    int n = PatchImports(proxy);
    Log("ASSET: %d proxy import(s) redirected%s", n, n >= 3 ? "" : " -> encrypted HopeGuard\\ files will not load");
}

// Imported by the patched d3d9.dll so that this DLL is initialized before the proxy's own DllMain.
extern "C" __declspec(dllexport) void HgInit() {}

static bool IsGameHost()
{
    uint8_t* base = (uint8_t*)GetModuleHandleA(nullptr);
    IMAGE_NT_HEADERS* nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    return nt->OptionalHeader.SizeOfImage >= 0x1000000;     // KnightOnLine.exe, not Option.exe
}

BOOL WINAPI DllMain(HINSTANCE h, DWORD reason, LPVOID)
{
    if (reason == DLL_PROCESS_ATTACH) {
        DisableThreadLibraryCalls(h);
        InstallAssets();
        if (IsGameHost() || GetEnvironmentVariableA("OPSGUARD_TEST", nullptr, 0))
            Install();
    }
    return TRUE;
}
