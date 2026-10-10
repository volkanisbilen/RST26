// The " key (Turkish Q layout: the key left of 1, DirectInput scan code 0x29) opened a native client menu for players.
// Nobody may get a game panel from it: the GM panel (gm_panel.cpp) is opened from WM_CHAR, which this does not
// touch, and typing " in the chat keeps working (the game's edits get WM_CHAR too).
// The game reads the keyboard through DirectInput 8 (dinput8.dll loaded at run time) and GetAsyncKeyState:
//  - IDirectInputDevice8 GetDeviceState (vtable 9): byte 0x29 of the 256-byte keyboard state is cleared
//  - IDirectInputDevice8 GetDeviceData (vtable 10): buffered events for offset 0x29 are dropped
//  - user32!GetAsyncKeyState: the virtual key of scan code 0x29 (current layout) reads as "up"
#define DIRECTINPUT_VERSION 0x0800
#include <windows.h>
#include <dinput.h>
#include <stdio.h>
#pragma comment(lib, "dxguid.lib")

void Log(const char* msg);

namespace
{
    const DWORD QUOTE_SCAN = 0x29;

    typedef HRESULT(STDMETHODCALLTYPE* FnState)(void*, DWORD, LPVOID);
    typedef HRESULT(STDMETHODCALLTYPE* FnData)(void*, DWORD, LPDIDEVICEOBJECTDATA, LPDWORD, DWORD);
    FnState g_oStateA = nullptr, g_oStateW = nullptr;
    FnData  g_oDataA = nullptr, g_oDataW = nullptr;

    void MaskState(HRESULT hr, DWORD cb, LPVOID data)
    {
        if (SUCCEEDED(hr) && cb == 256 && data) ((BYTE*)data)[QUOTE_SCAN] = 0;
    }
    void MaskData(HRESULT hr, DWORD cbObj, LPDIDEVICEOBJECTDATA rg, LPDWORD inOut)
    {
        if (FAILED(hr) || !rg || !inOut || cbObj < sizeof(DWORD)) return;
        DWORD n = *inOut, out = 0;
        for (DWORD i = 0; i < n; i++)
        {
            BYTE* src = (BYTE*)rg + (size_t)i * cbObj;
            if (*(DWORD*)src == QUOTE_SCAN) continue;  // dwOfs (mouse offsets are 0x00-0x13 without 0x29)
            if (out != i) memcpy((BYTE*)rg + (size_t)out * cbObj, src, cbObj);
            out++;
        }
        *inOut = out;
    }
    HRESULT STDMETHODCALLTYPE HkStateA(void* self, DWORD cb, LPVOID data) { HRESULT hr = g_oStateA(self, cb, data); MaskState(hr, cb, data); return hr; }
    HRESULT STDMETHODCALLTYPE HkStateW(void* self, DWORD cb, LPVOID data) { HRESULT hr = g_oStateW(self, cb, data); MaskState(hr, cb, data); return hr; }
    HRESULT STDMETHODCALLTYPE HkDataA(void* self, DWORD cb, LPDIDEVICEOBJECTDATA rg, LPDWORD io, DWORD fl) { HRESULT hr = g_oDataA(self, cb, rg, io, fl); MaskData(hr, cb, rg, io); return hr; }
    HRESULT STDMETHODCALLTYPE HkDataW(void* self, DWORD cb, LPDIDEVICEOBJECTDATA rg, LPDWORD io, DWORD fl) { HRESULT hr = g_oDataW(self, cb, rg, io, fl); MaskData(hr, cb, rg, io); return hr; }

    bool PatchSlot(void** vtbl, int slot, void* hook, void** orig)
    {
        if (vtbl[slot] == hook) return true;           // A and W can share one vtable
        DWORD old;
        if (!VirtualProtect(&vtbl[slot], sizeof(void*), PAGE_EXECUTE_READWRITE, &old)) return false;
        *orig = vtbl[slot];
        vtbl[slot] = hook;
        VirtualProtect(&vtbl[slot], sizeof(void*), old, &old);
        return true;
    }

    bool HookDevice(HMODULE dinput, bool wide)
    {
        typedef HRESULT(WINAPI* FnCreate)(HINSTANCE, DWORD, REFIID, LPVOID*, LPUNKNOWN);
        FnCreate create = (FnCreate)GetProcAddress(dinput, "DirectInput8Create");
        if (!create) return false;
        bool ok = false;
        if (wide)
        {
            IDirectInput8W* di = nullptr; IDirectInputDevice8W* dev = nullptr;
            if (SUCCEEDED(create(GetModuleHandleW(nullptr), DIRECTINPUT_VERSION, IID_IDirectInput8W, (LPVOID*)&di, nullptr)) && di)
            {
                if (SUCCEEDED(di->CreateDevice(GUID_SysKeyboard, &dev, nullptr)) && dev)
                {
                    void** vt = *(void***)dev;
                    void* sameAsA = (void*)HkStateA;
                    if (vt[9] == sameAsA) ok = true;   // shared with the A device, already hooked
                    else ok = PatchSlot(vt, 9, (void*)HkStateW, (void**)&g_oStateW) && PatchSlot(vt, 10, (void*)HkDataW, (void**)&g_oDataW);
                    dev->Release();
                }
                di->Release();
            }
        }
        else
        {
            IDirectInput8A* di = nullptr; IDirectInputDevice8A* dev = nullptr;
            if (SUCCEEDED(create(GetModuleHandleW(nullptr), DIRECTINPUT_VERSION, IID_IDirectInput8A, (LPVOID*)&di, nullptr)) && di)
            {
                if (SUCCEEDED(di->CreateDevice(GUID_SysKeyboard, &dev, nullptr)) && dev)
                {
                    void** vt = *(void***)dev;
                    ok = PatchSlot(vt, 9, (void*)HkStateA, (void**)&g_oStateA) && PatchSlot(vt, 10, (void*)HkDataA, (void**)&g_oDataA);
                    dev->Release();
                }
                di->Release();
            }
        }
        return ok;
    }

    // ---- user32!GetAsyncKeyState (hot-patchable prologue 8B FF 55 8B EC)
    typedef SHORT(WINAPI* FnAsync)(int);
    FnAsync g_oAsync = nullptr;
    volatile int g_quoteVk = 0;
    SHORT WINAPI HkAsync(int vk)
    {
        if (vk != 0 && vk == g_quoteVk) return 0;
        return g_oAsync(vk);
    }
    bool HookAsync()
    {
        BYTE* fn = (BYTE*)GetProcAddress(GetModuleHandleW(L"user32.dll"), "GetAsyncKeyState");
        static const BYTE kPro[5] = { 0x8B, 0xFF, 0x55, 0x8B, 0xEC };
        if (!fn || memcmp(fn, kPro, 5) != 0) return false;
        BYTE* tramp = (BYTE*)VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if (!tramp) return false;
        memcpy(tramp, kPro, 5);
        tramp[5] = 0xE9; *(DWORD*)(tramp + 6) = (DWORD)(fn + 5) - (DWORD)(tramp + 10);
        g_oAsync = (FnAsync)tramp;
        DWORD old;
        VirtualProtect(fn, 5, PAGE_EXECUTE_READWRITE, &old);
        fn[0] = 0xE9; *(DWORD*)(fn + 1) = (DWORD)HkAsync - (DWORD)(fn + 5);
        VirtualProtect(fn, 5, old, &old);
        FlushInstructionCache(GetCurrentProcess(), fn, 5);
        return true;
    }

    DWORD WINAPI InstallThread(LPVOID)
    {
        g_quoteVk = (int)MapVirtualKeyW(QUOTE_SCAN, MAPVK_VSC_TO_VK);
        bool async = HookAsync();
        HMODULE di = nullptr;
        for (int i = 0; i < 1200 && !(di = GetModuleHandleW(L"dinput8.dll")); i++) Sleep(100);   // loaded at run time
        if (!di) di = LoadLibraryW(L"dinput8.dll");
        bool a = di && HookDevice(di, false);
        bool w = di && HookDevice(di, true);
        char b[160]; sprintf_s(b, "QUOTEKEY: \" key hidden from the game (dinput A=%d W=%d, GetAsyncKeyState=%d vk=0x%02X)", a, w, async, g_quoteVk);
        Log(b);
        return 0;
    }
}

void QuoteKeyBlock_Init()
{
    CloseHandle(CreateThread(nullptr, 0, InstallThread, nullptr, 0, nullptr));
}

// the layout can change while the game runs (ProxyWndProc, WM_INPUTLANGCHANGE)
void QuoteKeyBlock_LayoutChanged()
{
    g_quoteVk = (int)MapVirtualKeyW(QUOTE_SCAN, MAPVK_VSC_TO_VK);
}
