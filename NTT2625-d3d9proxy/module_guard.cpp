// Anti-injection watch. Legitimate loaders (XIGNCODE, the game's own DLLs) finish loading
// within the first few seconds of the process; anything that loads a module into the game
// process AFTER that grace period is treated as injected (a cheat, a trainer, an overlay) and
// reported through the server handshake in client_integrity.cpp (sync at login, async the
// moment it is seen).
#include <windows.h>
#include <tlhelp32.h>
#include <cwctype>
#include <mutex>
#include <set>
#include <string>

void Log(const char* msg);
void Integrity_SendAsyncAlert();

namespace
{
    const DWORD GRACE_MS = 8000;   // let XIGNCODE / the game's own loaders finish first
    const DWORD POLL_MS = 4000;

    std::mutex g_mx;
    std::set<std::wstring> g_baseline;
    bool g_baselineReady = false;
    bool g_tampered = false;

    std::set<std::wstring> Snapshot()
    {
        std::set<std::wstring> mods;
        HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, GetCurrentProcessId());
        if (snap == INVALID_HANDLE_VALUE) return mods;
        MODULEENTRY32W me{};
        me.dwSize = sizeof(me);
        if (Module32FirstW(snap, &me))
        {
            do
            {
                std::wstring n = me.szModule;
                for (auto& c : n) c = (wchar_t)towlower((wint_t)c);
                mods.insert(n);
            } while (Module32NextW(snap, &me));
        }
        CloseHandle(snap);
        return mods;
    }

    void ScanOnce()
    {
        std::set<std::wstring> now = Snapshot();
        bool justTampered = false;
        std::wstring badModule;
        {
            std::lock_guard<std::mutex> lk(g_mx);
            if (!g_baselineReady)
            {
                g_baseline = now;
                g_baselineReady = true;
                return;
            }
            for (const std::wstring& n : now)
            {
                if (g_baseline.count(n)) continue;
                g_baseline.insert(n);
                if (!g_tampered)
                {
                    g_tampered = true;
                    justTampered = true;
                    badModule = n;
                }
            }
        }
        if (justTampered)
        {
            char buf[320];
            sprintf_s(buf, "INTEGRITY: module loaded after startup (possible injection): %ls", badModule.c_str());
            Log(buf);
            Integrity_SendAsyncAlert();
        }
    }

    DWORD WINAPI WatchThread(LPVOID)
    {
        Sleep(GRACE_MS);
        ScanOnce();   // first call after the grace period establishes the baseline
        for (;;)
        {
            Sleep(POLL_MS);
            ScanOnce();
        }
    }
}

void ModuleGuard_Init()
{
    CreateThread(nullptr, 0, WatchThread, nullptr, 0, nullptr);
}

bool ModuleGuard_Tampered()
{
    std::lock_guard<std::mutex> lk(g_mx);
    return g_tampered;
}
