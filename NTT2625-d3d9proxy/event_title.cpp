// Join window title for Juraid.
// The 2625 client has no join window for Juraid, so the server opens the Border Defence War one (event id 4,
// GameDefine.h TempleClientEvent). The game writes "Border Defense War" into its txt_title; while the event
// that is signing up is Juraid (the server names it in the EVREWARD packet) the title reads "Juraid Event".
//   CN3UIString vtable 0x01009EC8, slot 0xD8 / 4 = SetString(const std::string&)
#include <windows.h>
#include <stdio.h>
#include <string>

void Log(const char* msg);

namespace
{
    const DWORD VT_STRING = 0x01009EC8;
    const int SLOT_SETSTRING = 0xD8 / 4;
    typedef void(__thiscall* FnSet)(void*, const std::string*);

    FnSet g_orig = nullptr;
    void* volatile g_title = nullptr;       // the join window's title string, seen when the game sets it
    volatile bool g_juraid = false;

    bool IsBdwTitle(const std::string* s)
    {
        return s && (_stricmp(s->c_str(), "Border Defense War") == 0 || _stricmp(s->c_str(), "Border Defence War") == 0);
    }

    void __fastcall HkSetString(void* self, void*, const std::string* s)
    {
        if (IsBdwTitle(s))
        {
            if (g_title != self) { char b[64]; sprintf_s(b, "EVTITLE: join window title at %p", self); Log(b); }
            g_title = self;
            if (g_juraid)
            {
                std::string j = "Juraid Event";
                g_orig(self, &j);
                return;
            }
        }
        g_orig(self, s);
    }

    bool Alive(void* p)
    {
        MEMORY_BASIC_INFORMATION mbi;
        return p && VirtualQuery(p, &mbi, sizeof(mbi)) && mbi.State == MEM_COMMIT
            && !(mbi.Protect & (PAGE_NOACCESS | PAGE_GUARD)) && *(DWORD*)p == VT_STRING;
    }
}

// game thread (recv hook): the event that is signing up now
void EventTitle_SetJuraid(bool on)
{
    g_juraid = on;
    void* t = g_title;
    if (!g_orig || !Alive(t)) return;
    std::string s = on ? "Juraid Event" : "Border Defense War";
    g_orig(t, &s);
}

void EventTitle_Init()
{
    DWORD* slot = (DWORD*)VT_STRING + SLOT_SETSTRING;
    MEMORY_BASIC_INFORMATION mbi;
    if (!VirtualQuery(slot, &mbi, sizeof(mbi)) || mbi.State != MEM_COMMIT || *slot < 0x00401000 || *slot >= 0x00F93000)
    {
        Log("EVTITLE: CN3UIString vtable not where expected, Juraid title off");
        return;
    }
    g_orig = (FnSet)*slot;
    DWORD old;
    VirtualProtect(slot, 4, PAGE_READWRITE, &old);
    *slot = (DWORD)HkSetString;
    VirtualProtect(slot, 4, old, &old);
    Log("EVTITLE: SetString hook installed");
}
