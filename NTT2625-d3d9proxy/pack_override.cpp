// Pack overrides: replace files inside the game's packs (Object / Item / fx / UI / Snd .src + .hdr, Path.ini [Dir])
// without shipping a new multi-hundred-MB .src. A loose copy next to the pack is NOT used by the client (the pack wins),
// so the bytes are swapped while the game reads them:
//   HopeGuard\pack_override\<pack dir>\<name as in the .hdr>   e.g. HopeGuard\pack_override\fx\billboard\gng\empires_p_128_0000.dxt
// At start-up every such file is matched with its .hdr entry (offset, size); it must not be larger than the entry (the
// game reads exactly the entry size; the rest is zero padded). KernelBase!ReadFile is hooked: a read from a pack .src
// handle (recognised by its path, once per handle) gets the override bytes over the overlapping part.
// First use (2026-10-09): the [Empires Emblem] item (931719000) shows the HopeGuard emblem (build_hg_emblem.py).
#include <windows.h>
#include <stdio.h>
#include <string>
#include <vector>
#include <unordered_map>
#include <algorithm>

void Log(const char* msg);

namespace
{
    struct Override { int pack; LONGLONG off; std::vector<BYTE> bytes; std::string name; };
    std::vector<Override> g_over;
    std::vector<std::wstring> g_packSrc;              // lower-case path tails "\fx\fx.src" of packs with overrides
    std::unordered_map<HANDLE, int> g_handles;        // handle -> pack index (-1 = not a pack)
    SRWLOCK g_lock = SRWLOCK_INIT;

    typedef BOOL(WINAPI* FnRead)(HANDLE, LPVOID, DWORD, LPDWORD, LPOVERLAPPED);
    typedef BOOL(WINAPI* FnClose)(HANDLE);
    FnRead g_oRead = nullptr;
    FnClose g_oClose = nullptr;

    std::string GameDir() { char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH); *(strrchr(p, '\\') + 1) = 0; return p; }
    std::string Lower(std::string s) { for (auto& c : s) c = (char)tolower((unsigned char)c); return s; }

    bool ReadAll(const std::string& path, std::vector<BYTE>& out)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, path.c_str(), "rb") != 0 || !f) return false;
        fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
        out.resize(n > 0 ? n : 0);
        bool ok = n <= 0 || fread(out.data(), 1, n, f) == (size_t)n;
        fclose(f);
        return ok;
    }

    void Walk(const std::string& dir, const std::string& rel, std::vector<std::string>& out)
    {
        WIN32_FIND_DATAA fd; HANDLE h = FindFirstFileA((dir + "\\*").c_str(), &fd);
        if (h == INVALID_HANDLE_VALUE) return;
        do
        {
            if (fd.cFileName[0] == '.') continue;
            std::string r = rel.empty() ? fd.cFileName : rel + "\\" + fd.cFileName;
            if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) Walk(dir + "\\" + fd.cFileName, r, out);
            else out.push_back(r);
        } while (FindNextFileA(h, &fd));
        FindClose(h);
    }

    void LoadOverrides()
    {
        std::string game = GameDir(), root = game + "HopeGuard\\pack_override";
        static const char* kPacks[] = { "object", "item", "fx", "ui", "snd" };
        for (const char* pk : kPacks)
        {
            std::vector<std::string> files;
            Walk(root + "\\" + pk, "", files);
            if (files.empty()) continue;
            std::vector<BYTE> hdr;
            if (!ReadAll(game + pk + "\\" + pk + ".hdr", hdr) || hdr.size() < 8) { Log("PACKOVR: no .hdr"); continue; }
            std::unordered_map<std::string, std::pair<DWORD, DWORD>> entries;
            for (size_t o = 8; o + 2 <= hdr.size();)
            {
                WORD l = *(WORD*)&hdr[o];
                if (o + 2 + l + 8 > hdr.size()) break;
                std::string n((const char*)&hdr[o + 2], l);
                entries[Lower(n)] = { *(DWORD*)&hdr[o + 2 + l], *(DWORD*)&hdr[o + 2 + l + 4] };
                o += 2 + l + 8;
            }
            int packIdx = (int)g_packSrc.size();
            int used = 0;
            for (auto& rel : files)
            {
                char b[300];
                auto it = entries.find(Lower(rel));
                if (it == entries.end()) { sprintf_s(b, "PACKOVR: %s\\%s is not in the pack", pk, rel.c_str()); Log(b); continue; }
                Override ov; ov.pack = packIdx; ov.off = it->second.first; ov.name = rel;
                if (!ReadAll(root + "\\" + pk + "\\" + rel, ov.bytes)) continue;
                if (ov.bytes.size() > it->second.second)
                {
                    sprintf_s(b, "PACKOVR: %s\\%s is %u bytes, the pack slot %u: skipped", pk, rel.c_str(), (unsigned)ov.bytes.size(), it->second.second); Log(b); continue;
                }
                ov.bytes.resize(it->second.second, 0);
                g_over.push_back(std::move(ov)); used++;
                sprintf_s(b, "PACKOVR: %s\\%s -> offset %u size %u", pk, rel.c_str(), it->second.first, it->second.second); Log(b);
            }
            if (used)
            {
                std::string tail = std::string("\\") + pk + "\\" + pk + ".src";
                g_packSrc.push_back(std::wstring(tail.begin(), tail.end()));
            }
        }
    }

    int PackOf(HANDLE h)
    {
        AcquireSRWLockShared(&g_lock);
        auto it = g_handles.find(h); int r = it == g_handles.end() ? -2 : it->second;
        ReleaseSRWLockShared(&g_lock);
        if (r != -2) return r;
        r = -1;
        wchar_t p[MAX_PATH * 2];
        DWORD n = GetFinalPathNameByHandleW(h, p, MAX_PATH * 2, FILE_NAME_NORMALIZED);
        if (n > 0 && n < MAX_PATH * 2 && GetFileType(h) == FILE_TYPE_DISK)
        {
            std::wstring s(p, n); for (auto& c : s) c = (wchar_t)towlower(c);
            for (size_t i = 0; i < g_packSrc.size(); i++)
                if (s.size() >= g_packSrc[i].size() && s.compare(s.size() - g_packSrc[i].size(), g_packSrc[i].size(), g_packSrc[i]) == 0) r = (int)i;
        }
        AcquireSRWLockExclusive(&g_lock); g_handles[h] = r; ReleaseSRWLockExclusive(&g_lock);
        return r;
    }

    BOOL WINAPI HkRead(HANDLE h, LPVOID buf, DWORD n, LPDWORD read, LPOVERLAPPED ov)
    {
        int pack = (buf && n) ? PackOf(h) : -1;
        LONGLONG pos = -1;
        if (pack >= 0)
        {
            if (ov) pos = (LONGLONG)ov->Offset | ((LONGLONG)ov->OffsetHigh << 32);
            else { LARGE_INTEGER z = {}, cur; if (SetFilePointerEx(h, z, &cur, FILE_CURRENT)) pos = cur.QuadPart; }
        }
        BOOL ok = g_oRead(h, buf, n, read, ov);
        if (ok && pos >= 0)
        {
            DWORD got = read ? *read : n;
            for (const Override& o : g_over)
            {
                if (o.pack != pack) continue;
                LONGLONG a = (std::max)(pos, o.off), b = (std::min)(pos + (LONGLONG)got, o.off + (LONGLONG)o.bytes.size());
                if (a < b) memcpy((BYTE*)buf + (a - pos), o.bytes.data() + (a - o.off), (size_t)(b - a));
            }
        }
        return ok;
    }
    BOOL WINAPI HkClose(HANDLE h)
    {
        AcquireSRWLockExclusive(&g_lock); g_handles.erase(h); ReleaseSRWLockExclusive(&g_lock);
        return g_oClose(h);
    }

    // hot-patchable prologue 8B FF 55 8B EC -> jmp hook, trampoline = prologue + jmp back
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

// d3d9proxy start-up (before the game reads its packs, handles opened earlier are recognised by path anyway)
void PackOverride_Init()
{
    LoadOverrides();
    if (g_over.empty()) return;
    g_oClose = (FnClose)HookHot("CloseHandle", (void*)HkClose);
    g_oRead = (FnRead)HookHot("ReadFile", (void*)HkRead);
    char b[120]; sprintf_s(b, "PACKOVR: %u override(s), ReadFile hook %s", (unsigned)g_over.size(), g_oRead ? "on" : "FAILED");
    Log(b);
}
