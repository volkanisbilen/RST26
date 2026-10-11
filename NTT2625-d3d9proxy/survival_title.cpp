// Registration window title swap. The client's native survival registration window (CUISurvivalEnter,
// re_survival_enter.uif) has a fixed "Manes' Survival" title in its str_title control. Other events that
// reuse the window (Ultima's Dungeon) send E9 FA [u16 len][title] right before the window packet and this
// sets the control's text. CSurvival = *(*(0x011158FC) + 0x68C); its window is at CSurvival+0x130 and the
// window's child list at +176 (nodes: next, prev, child*; child name = std::string at child+88).
#include <windows.h>
#include <string.h>
#include <stdio.h>

void Log(const char* msg);

namespace
{
    const DWORD KO_PTR_PROC = 0x011158FC;     // CGameProcMain*

    struct GameStr { char buf[16]; DWORD size; DWORD res; };   // MSVC std::string (SSO) layout of the game

    const char* ChildName(DWORD child, char* tmp)
    {
        DWORD cap = *(DWORD*)(child + 88 + 20);
        const char* name = cap > 15 ? *(const char**)(child + 88) : (const char*)(child + 88);
        strncpy_s(tmp, 64, name, _TRUNCATE);
        return tmp;
    }

    bool SetTitle(const char* title, size_t n)
    {
        if (n == 0 || n > 15) return false;     // inline buffer only
        DWORD proc = *(DWORD*)KO_PTR_PROC;
        if (!proc) return false;
        DWORD survival = *(DWORD*)(proc + 0x68C);
        if (!survival) return false;
        DWORD window = *(DWORD*)(survival + 0x130);
        if (!window) return false;
        DWORD list = *(DWORD*)(window + 176);
        if (!list) return false;
        DWORD node = *(DWORD*)list;
        for (int guard = 0; node != list && node && guard < 256; guard++, node = *(DWORD*)node)
        {
            DWORD child = *(DWORD*)(node + 8);
            if (!child) continue;
            char name[64];
            if (_stricmp(ChildName(child, name), "str_title") != 0) continue;
            GameStr s = {};
            memcpy(s.buf, title, n);
            s.size = (DWORD)n;
            s.res = 15;
            typedef void(__thiscall * SetString)(void*, void*);
            SetString fn = (SetString)(*(DWORD**)child)[216 / 4];
            fn((void*)child, &s);
            return true;
        }
        return false;
    }
}

void SurvivalTitle_OnRecv(const BYTE* p, size_t len)
{
    if (len < 5 || p[0] != 0xE9 || p[1] != 0xFA) return;
    size_t n = *(const WORD*)(p + 2);
    if (n == 0 || 4 + n > len) return;
    __try
    {
        bool ok = SetTitle((const char*)p + 4, n);
        char msg[96];
        sprintf_s(msg, "SurvivalTitle: '%.*s' -> %s", (int)n, (const char*)p + 4, ok ? "set" : "window not ready");
        Log(msg);
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("SurvivalTitle: exception (ignored)"); }
}
