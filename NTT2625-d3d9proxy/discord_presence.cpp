// Discord Rich Presence ("Playing <application name>" with two lines and the play time), as cyber001 CyberACS does it:
// the open-source discord-rpc (discord_rpc\, copied from cyber's DiscordSDK) talks to the local Discord app over its pipe.
// Server: GameServer XGuard.cpp SendDiscordPresence, GameServer.ini [DISCORD] APP_ID / IMAGE / DETAILS, sent at game start,
// zone loaded and level up:  E9 F6 [str app id][str image][str details][str state]   (u16 length, CP1254)
// The last application id is kept in HopeGuard\discord.dat so the login screen shows the presence too.
#include <windows.h>
#include <stdio.h>
#include <string>
#include <time.h>
#include "discord_rpc/include/discord_rpc.h"

void Log(const char* msg);

namespace
{
    const BYTE SUBOP = 0xF6;
    CRITICAL_SECTION g_lock;
    bool g_inited = false, g_lockReady = false;
    std::string g_app, g_image = "logo", g_details = "HopeGuard 26XX", g_state;
    int64_t g_start = 0;

    std::string Utf8(const std::string& cp1254)
    {
        if (cp1254.empty()) return std::string();
        int n = MultiByteToWideChar(1254, 0, cp1254.data(), (int)cp1254.size(), nullptr, 0);
        std::wstring w(n, L'\0'); MultiByteToWideChar(1254, 0, cp1254.data(), (int)cp1254.size(), &w[0], n);
        int m = WideCharToMultiByte(CP_UTF8, 0, w.data(), (int)w.size(), nullptr, 0, nullptr, nullptr);
        std::string s(m, '\0'); WideCharToMultiByte(CP_UTF8, 0, w.data(), (int)w.size(), &s[0], m, nullptr, nullptr);
        return s;
    }
    std::string DatPath()
    {
        char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH);
        strcpy_s(strrchr(p, '\\') + 1, 48, "HopeGuard\\discord.dat");
        return p;
    }
    // caller holds g_lock
    void Push()
    {
        if (g_app.empty()) return;
        if (!g_inited)
        {
            DiscordEventHandlers h = {};
            Discord_Initialize(g_app.c_str(), &h, 0, nullptr);
            g_inited = true;
        }
        DiscordRichPresence p = {};
        p.details = g_details.c_str();
        p.state = g_state.c_str();
        p.startTimestamp = g_start;
        p.largeImageKey = g_image.c_str();
        p.largeImageText = g_details.c_str();
        Discord_UpdatePresence(&p);
    }
}

// d3d9proxy start-up
void Discord_InitProxy()
{
    InitializeCriticalSection(&g_lock); g_lockReady = true;
    g_start = (int64_t)time(nullptr);
    char line[512] = {};
    FILE* f = nullptr;
    if (fopen_s(&f, DatPath().c_str(), "rb") == 0 && f)
    {
        size_t n = fread(line, 1, sizeof(line) - 1, f); fclose(f); line[n] = 0;
        // app id \n image \n details
        char* ctx = nullptr;
        char* a = strtok_s(line, "\r\n", &ctx); char* b = strtok_s(nullptr, "\r\n", &ctx); char* c = strtok_s(nullptr, "\r\n", &ctx);
        if (a && *a) g_app = a;
        if (b && *b) g_image = b;
        if (c && *c) g_details = c;
    }
    if (g_app.empty()) { Log("DISCORD: no application id yet (comes from the server)"); return; }
    EnterCriticalSection(&g_lock);
    g_state = u8"Giriş ekranında";
    Push();
    LeaveCriticalSection(&g_lock);
    Log("DISCORD: presence started (login screen)");
}

// recv hook (game thread): E9 F6 ...
void Discord_OnRecv(const BYTE* p, size_t len)
{
    if (!g_lockReady || len < 3 || p[0] != 0xE9 || p[1] != SUBOP) return;
    size_t at = 2;
    auto str = [&](std::string& v) {
        if (at + 2 > len) return false;
        WORD n = *(const WORD*)(p + at); at += 2;
        if (n > 256 || at + n > len) return false;
        v.assign((const char*)p + at, n); at += n; return true;
    };
    std::string app, image, details, state;
    if (!str(app) || !str(image) || !str(details) || !str(state) || app.empty()) return;
    EnterCriticalSection(&g_lock);
    bool changed = app != g_app || image != g_image || Utf8(details) != g_details;
    if (g_inited && app != g_app) { Discord_Shutdown(); g_inited = false; }
    g_app = app; g_image = image; g_details = Utf8(details); g_state = Utf8(state);
    Push();
    LeaveCriticalSection(&g_lock);
    if (changed)
    {
        FILE* f = nullptr;
        if (fopen_s(&f, DatPath().c_str(), "wb") == 0 && f) { fprintf(f, "%s\n%s\n%s\n", app.c_str(), image.c_str(), g_details.c_str()); fclose(f); }
    }
}
