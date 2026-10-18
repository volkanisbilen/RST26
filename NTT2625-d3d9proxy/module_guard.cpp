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
#include <vector>

void Log(const char* msg);
void Integrity_SendAsyncAlert();

namespace
{
    // Long on purpose: XIGNCODE and plenty of ordinary gaming software (GPU vendor overlays,
    // Discord, Steam, RTSS/Afterburner...) hook into a D3D9 game on its first Present() call,
    // which can land well after process start on a slow loading screen -- a short grace period
    // here reported perfectly normal players' software as "injected" and kicked them for it.
    const DWORD GRACE_MS = 90000;
    const DWORD POLL_MS = 5000;

    // Common, legitimate modules that hook/inject into most games' processes. Never flagged as
    // tampering even if they load after the grace period. Substring match, lowercase.
    const wchar_t* kKnownOverlays[] = {
        L"nvngx", L"nvspcap", L"nvd3d", L"nvapi", L"geforce", L"nvidia",
        L"rtsshooks", L"rtss", L"msiafterburner",
        L"discordhook", L"discord",
        L"gameoverlayrenderer", L"steam",
        L"razerds", L"razer", L"rzchromasdk",
        L"corsair", L"icue", L"logitech", L"lghub",
        L"xboxgipsvc", L"gamebar", L"gamingoverlay",
        L"origin", L"eadesktop", L"battle.net", L"battlenet",
        L"wacom", L"asustek", L"armoury",
        L"reshade", L"specialk",
        // Windows itself, loaded lazily on first use (TLS/certificate checks, name resolution,
        // the HTTPS the game and this DLL's own patch system do) -- not overlays, but just as
        // legitimate and just as likely to show up well after the grace period.
        L"cryptnet", L"cryptsp", L"rsaenh", L"schannel", L"ncrypt", L"ncryptsslp",
        L"mswsock", L"iphlpapi", L"winhttp", L"wininet", L"urlmon", L"dnsapi",
        L"profapi", L"cryptbase", L"sspicli", L"windows.storage",
    };

    bool IsKnownOverlay(const std::wstring& name)
    {
        for (auto w : kKnownOverlays)
            if (name.find(w) != std::wstring::npos)
                return true;
        return false;
    }

    std::mutex g_mx;
    std::set<std::wstring> g_baseline;
    bool g_baselineReady = false;
    bool g_tampered = false;

    // Windows' own DLLs (hundreds of them, many loaded lazily on first use -- crypto, name
    // resolution, shell bits) live in System32/SysWOW64 and get a free pass by location instead
    // of chasing an endless name list. A module the game itself ships gets a free pass too.
    std::wstring g_systemDir, g_wow64Dir, g_gameDir;

    bool IsTrustedLocation(const std::wstring& path)
    {
        std::wstring low = path;
        for (auto& c : low) c = (wchar_t)towlower((wint_t)c);
        auto under = [&](const std::wstring& dir) { return !dir.empty() && low.rfind(dir, 0) == 0; };
        return under(g_systemDir) || under(g_wow64Dir) || under(g_gameDir);
    }

    struct ModuleInfo { std::wstring name, path; };

    std::vector<ModuleInfo> Snapshot()
    {
        std::vector<ModuleInfo> mods;
        HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, GetCurrentProcessId());
        if (snap == INVALID_HANDLE_VALUE) return mods;
        MODULEENTRY32W me{};
        me.dwSize = sizeof(me);
        if (Module32FirstW(snap, &me))
        {
            do
            {
                std::wstring n = me.szModule, p = me.szExePath;
                for (auto& c : n) c = (wchar_t)towlower((wint_t)c);
                mods.push_back({ n, p });
            } while (Module32NextW(snap, &me));
        }
        CloseHandle(snap);
        return mods;
    }

    void ScanOnce()
    {
        std::vector<ModuleInfo> now = Snapshot();
        bool justTampered = false;
        std::wstring badModule;
        {
            std::lock_guard<std::mutex> lk(g_mx);
            if (!g_baselineReady)
            {
                for (auto& m : now) g_baseline.insert(m.name);
                g_baselineReady = true;
                return;
            }
            for (auto& m : now)
            {
                if (g_baseline.count(m.name)) continue;
                g_baseline.insert(m.name);
                if (IsKnownOverlay(m.name) || IsTrustedLocation(m.path)) continue;
                if (!g_tampered)
                {
                    g_tampered = true;
                    justTampered = true;
                    badModule = m.name + L" (" + m.path + L")";
                }
            }
        }
        if (justTampered)
        {
            char buf[480];
            sprintf_s(buf, "INTEGRITY: module loaded after startup (possible injection): %ls", badModule.c_str());
            Log(buf);
            Integrity_SendAsyncAlert();
        }
    }

    void InitTrustedDirs()
    {
        wchar_t buf[MAX_PATH];
        if (GetSystemDirectoryW(buf, MAX_PATH)) { g_systemDir = buf; for (auto& c : g_systemDir) c = (wchar_t)towlower((wint_t)c); }
        if (GetSystemWow64DirectoryW(buf, MAX_PATH)) { g_wow64Dir = buf; for (auto& c : g_wow64Dir) c = (wchar_t)towlower((wint_t)c); }
        if (GetModuleFileNameW(nullptr, buf, MAX_PATH))
        {
            std::wstring self = buf;
            size_t slash = self.find_last_of(L'\\');
            if (slash != std::wstring::npos) self.resize(slash);
            g_gameDir = self;
            for (auto& c : g_gameDir) c = (wchar_t)towlower((wint_t)c);
        }
    }

    DWORD WINAPI WatchThread(LPVOID)
    {
        InitTrustedDirs();
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
