#define DIRECTINPUT_VERSION 0x0800
#include "stdafx.h"
#include <tlhelp32.h>
#include "resource.h"
#include <d3d9.h>
#include <d3dx9core.h>
#include <d3dx9tex.h>
#include <dinput.h>
#include <tchar.h>
#include "LauncherEngine.h"
#include "CRC.h"
#include "MD5.h"
#include "sha.hpp"
#include <dwmapi.h>
#include "gif.h"
#include "GDIHelper.h"
#pragma comment(lib, "Dwmapi.lib")
#pragma comment(lib, "Msimg32.lib")
#pragma comment(lib, "d3d9.lib")
// D3DX import library removed: functions are loaded dynamically for x86/x64 compatibility.
#pragma warning(disable:4244)
#pragma warning(disable:4129)

namespace
{
    HMODULE g_hD3DX9 = NULL;

    typedef HRESULT(WINAPI* PFN_D3DXCreateSprite)(LPDIRECT3DDEVICE9, LPD3DXSPRITE*);
    typedef HRESULT(WINAPI* PFN_D3DXCreateFontA)(LPDIRECT3DDEVICE9, INT, UINT, UINT, UINT, BOOL, DWORD, DWORD, DWORD, DWORD, LPCSTR, LPD3DXFONT*);
    typedef HRESULT(WINAPI* PFN_D3DXCreateTextureFromFileInMemoryEx)(LPDIRECT3DDEVICE9, LPCVOID, UINT, UINT, UINT, UINT, DWORD, D3DFORMAT, D3DPOOL, DWORD, DWORD, D3DCOLOR, D3DXIMAGE_INFO*, PALETTEENTRY*, LPDIRECT3DTEXTURE9*);

    PFN_D3DXCreateSprite g_pD3DXCreateSprite = NULL;
    PFN_D3DXCreateFontA g_pD3DXCreateFontA = NULL;
    PFN_D3DXCreateTextureFromFileInMemoryEx g_pD3DXCreateTextureFromFileInMemoryEx = NULL;

    bool LoadD3DX9Runtime()
    {
        if (g_hD3DX9 != NULL)
            return true;

        const char* dllNames[] =
        {
            "d3dx9_43.dll", "d3dx9_42.dll", "d3dx9_41.dll", "d3dx9_40.dll",
            "d3dx9_39.dll", "d3dx9_38.dll", "d3dx9_37.dll", "d3dx9_36.dll",
            "d3dx9_35.dll", "d3dx9_34.dll", "d3dx9_33.dll", "d3dx9_32.dll",
            "d3dx9_31.dll", "d3dx9_30.dll", "d3dx9_29.dll", "d3dx9_28.dll",
            "d3dx9_27.dll", "d3dx9_26.dll", "d3dx9_25.dll", "d3dx9_24.dll"
        };

        for (const char* dllName : dllNames)
        {
            g_hD3DX9 = LoadLibraryA(dllName);
            if (g_hD3DX9 != NULL)
                break;
        }

        if (g_hD3DX9 == NULL)
            return false;

        g_pD3DXCreateSprite = reinterpret_cast<PFN_D3DXCreateSprite>(GetProcAddress(g_hD3DX9, "D3DXCreateSprite"));
        g_pD3DXCreateFontA = reinterpret_cast<PFN_D3DXCreateFontA>(GetProcAddress(g_hD3DX9, "D3DXCreateFontA"));
        g_pD3DXCreateTextureFromFileInMemoryEx = reinterpret_cast<PFN_D3DXCreateTextureFromFileInMemoryEx>(GetProcAddress(g_hD3DX9, "D3DXCreateTextureFromFileInMemoryEx"));

        return g_pD3DXCreateSprite != NULL &&
               g_pD3DXCreateFontA != NULL &&
               g_pD3DXCreateTextureFromFileInMemoryEx != NULL;
    }

    HRESULT JstD3DXCreateSprite(LPDIRECT3DDEVICE9 device, LPD3DXSPRITE* sprite)
    {
        if (!LoadD3DX9Runtime())
            return E_FAIL;
        return g_pD3DXCreateSprite(device, sprite);
    }

    HRESULT JstD3DXCreateFontA(LPDIRECT3DDEVICE9 device, INT height, UINT width, UINT weight, UINT mipLevels, BOOL italic,
        DWORD charSet, DWORD outputPrecision, DWORD quality, DWORD pitchAndFamily, LPCSTR faceName, LPD3DXFONT* font)
    {
        if (!LoadD3DX9Runtime())
            return E_FAIL;
        return g_pD3DXCreateFontA(device, height, width, weight, mipLevels, italic, charSet, outputPrecision, quality, pitchAndFamily, faceName, font);
    }

    HRESULT JstD3DXCreateTextureFromFileInMemoryEx(LPDIRECT3DDEVICE9 device, LPCVOID srcData, UINT srcDataSize,
        UINT width, UINT height, UINT mipLevels, DWORD usage, D3DFORMAT format, D3DPOOL pool, DWORD filter,
        DWORD mipFilter, D3DCOLOR colorKey, D3DXIMAGE_INFO* srcInfo, PALETTEENTRY* palette, LPDIRECT3DTEXTURE9* texture)
    {
        if (!LoadD3DX9Runtime())
            return E_FAIL;
        return g_pD3DXCreateTextureFromFileInMemoryEx(device, srcData, srcDataSize, width, height, mipLevels, usage, format, pool, filter, mipFilter, colorKey, srcInfo, palette, texture);
    }
}
static int iW = 802;
static int iH = 594;
static int iWOff = 0;
CheckSum Check;
MD5	md5;

// Data
static LPDIRECT3D9              g_pD3D = NULL;
static LPDIRECT3DDEVICE9        g_pd3dDevice = NULL;
static D3DPRESENT_PARAMETERS    g_d3dpp = {};


thyke_Test* thyke_t = NULL;
// Font
LPD3DXFONT m_font = NULL;
RECT TextStatePos;
RECT TextNoticePos;
RECT TextPagePos;
int NoticeOffsetY = 0;
int TextStateBaseX = 0;

// Textures
IDirect3DTexture9* LauncherBackgroundTexture = NULL;

IDirect3DTexture9* StartButtonTexture = NULL;
IDirect3DTexture9* StartButtonHoverTexture = NULL;
IDirect3DTexture9* StartButtonDownTexture = NULL;

IDirect3DTexture9* HomePageButtonTexture = NULL;
IDirect3DTexture9* HomePageButtonHoverTexture = NULL;
IDirect3DTexture9* HomePageButtonDownTexture = NULL;

IDirect3DTexture9* SettingsButtonTexture = NULL;
IDirect3DTexture9* SettingsButtonDownTexture = NULL;
IDirect3DTexture9* SettingsButtonHoverTexture = NULL;

IDirect3DTexture9* CloseButtonTexture = NULL;
IDirect3DTexture9* CloseButtonHoverTexture = NULL;
IDirect3DTexture9* CloseButtonDownTexture = NULL;

//IDirect3DTexture9* DiscordButtonTexture = NULL;
//IDirect3DTexture9* DiscordButtonHoverTexture = NULL;
//IDirect3DTexture9* DiscordButtonDownTexture = NULL;
//
//IDirect3DTexture9* ForumButtonTexture = NULL;
//IDirect3DTexture9* ForumButtonHoverTexture = NULL;
//IDirect3DTexture9* ForumButtonDownTexture = NULL;
//
//IDirect3DTexture9* FacebookButtonTexture = NULL;
//IDirect3DTexture9* FacebookButtonHoverTexture = NULL;
//IDirect3DTexture9* FacebookButtonDownTexture = NULL;

IDirect3DTexture9* ProgressTexture = NULL;
IDirect3DTexture9* ProgressFillTexture = NULL;


// Sprites
LPD3DXSPRITE LauncherSprite = NULL;

// Vectors
D3DXVECTOR3 LauncherBackgorundPosition(0, 0, 0);
D3DXVECTOR3 StartButtonPosition(608.0f + iWOff, 509.0f, 0);
D3DXVECTOR3 HomePageButtonPosition(23.0f + iWOff, 511.0f, 0);
D3DXVECTOR3 SettingsButtonPosition(161.0f + iWOff, 511.0f, 0);
D3DXVECTOR3 CloseButtonPosition(765.0f + iWOff, 3.0f, 0);
//D3DXVECTOR3 DiscordButtonPosition(608.0f + iWOff, 509.0f, 0);
//D3DXVECTOR3 ForumButtonPosition(23.0f + iWOff, 511.0f, 0);
//D3DXVECTOR3 FacebookButtonPosition(161.0f + iWOff, 511.0f, 0);
D3DXVECTOR3 ProgressPosition(24.0f + iWOff, 557.0f, 0);
D3DXVECTOR3 ProgressfillPosition(12.0f + iWOff, 368.0f, 0);
RECT pbFill;

// Surfaces
D3DSURFACE_DESC StartButtonSurface;
D3DSURFACE_DESC HomePageButtonSurface;
D3DSURFACE_DESC SettingsButtonSurface;
D3DSURFACE_DESC CloseButtonSurface;
//D3DSURFACE_DESC DiscordButtonSurface;
//D3DSURFACE_DESC ForumButtonSurface;
//D3DSURFACE_DESC FacebookButtonSurface;
D3DSURFACE_DESC ProgressSurface;
// States
enum ButtonState
{
    STATE_NORMAL = 0,
    STATE_HOVER,
    STATE_DOWN,
    STATE_UP
};
// Statics
#define COL_N D3DCOLOR_ARGB(255, 200, 200, 200)
#define COL_H D3DCOLOR_ARGB(255, 255, 255, 255)
#define COL_D D3DCOLOR_ARGB(255, 150, 150, 150)
#define COL_X D3DCOLOR_ARGB(255, 80, 80, 80)
ButtonState states[7] = {};
static ButtonState lastStartState = STATE_NORMAL;
static ButtonState lastHomepageState = STATE_NORMAL;
static ButtonState lastSettingsState = STATE_NORMAL;
static ButtonState lastCloseState = STATE_NORMAL;
//static ButtonState lastDiscordState = STATE_NORMAL;
//static ButtonState lastForumState = STATE_NORMAL;
//static ButtonState lastFacebookState = STATE_NORMAL;

static HCURSOR hCursorNormal;
static HCURSOR hCursorHand;
static HCURSOR hCursorClick;
HWND mainWindow;
HMODULE mainInstance;
size_t sayfalar;
size_t sayfa;

// Engine
Launcher* Engine;
int loadCount = 0;
std::string launcherdir;
std::string Address_HomePage;
//std::string Address_Discord;
//std::string Address_Forum;
//std::string Address_Facebook;
CHAR WP[MAXCHAR];

bool CreateDeviceD3D(HWND hWnd);
void CleanupDeviceD3D();
void ResetDevice();

static void _string_format(const std::string fmt, std::string* result, va_list args)
{
    char buffer[1024];
    _vsnprintf(buffer, sizeof(buffer), fmt.c_str(), args);
    *result = buffer;
}

static std::string string_format(const std::string fmt, ...)
{
    std::string result;
    va_list ap;

    va_start(ap, fmt);
    _string_format(fmt, &result, ap);
    va_end(ap);

    return result;
}

extern std::string hashfile(const std::string filename)
{
    SHA1 checksum;
    return checksum.from_file(filename);
}

std::string GetDLLDir()
{
    char result[MAX_PATH];
    return std::string(result, GetModuleFileName(NULL, result, MAX_PATH));
}

std::string GetDir()
{
    std::string dir = GetDLLDir();
   dir.replace(dir.size() - 13, 13, "");
    return dir;
}

std::string FileDirGet(std::string FileName)
{
    std::string x;
    std::string name = xorstr("%s/%s");
    x = string_format(name, GetDir().c_str(), FileName.c_str());
    return x;
}
void HSACSXEncrypt()
{
  
}

void HSACSXEncryptt()
{
   
}

void WebLinkAdded()
{
    Address_HomePage = "https://www.knightgenie.com/";
    //Address_Forum = "#";
    //Address_Facebook = "#";
    //Address_Discord = "#";
}

int WINAPI SocketSystem()
{
    Engine->Start();
    return 1;
}

// exe icindeki kaynaklardan
static HRESULT LoadTextureFromEmbeddedResource(UINT resourceId, IDirect3DTexture9** targetTexture)
{
    if (targetTexture == NULL)
        return E_POINTER;

    *targetTexture = NULL;

    HMODULE hModule = mainInstance != NULL ? mainInstance : GetModuleHandle(NULL);
    HRSRC resourceInfo = FindResourceA(hModule, MAKEINTRESOURCEA(resourceId), MAKEINTRESOURCEA(10));
    if (resourceInfo == NULL)
        return E_FAIL;

    DWORD resourceSize = SizeofResource(hModule, resourceInfo);
    if (resourceSize == 0)
        return E_FAIL;

    HGLOBAL resourceData = LoadResource(hModule, resourceInfo);
    if (resourceData == NULL)
        return E_FAIL;

    const void* resourceBytes = LockResource(resourceData);
    if (resourceBytes == NULL)
        return E_FAIL;

    return JstD3DXCreateTextureFromFileInMemoryEx(
        g_pd3dDevice,
        resourceBytes,
        resourceSize,
        D3DX_DEFAULT_NONPOW2,
        D3DX_DEFAULT_NONPOW2,
        0,
        0,
        D3DFMT_UNKNOWN,
        D3DPOOL_MANAGED,
        D3DX_FILTER_NONE,
        D3DX_DEFAULT,
        D3DCOLOR_ARGB(128, 128, 128, 128),
        0,
        0,
        targetTexture
    );
}

bool LoadTextures()
{
    if (FAILED(JstD3DXCreateSprite(g_pd3dDevice, &LauncherSprite)) || LauncherSprite == NULL)
    {
        MessageBoxA(mainWindow, "D3DX9 runtime could not be loaded. Please install DirectX 9.0c runtime or place the correct d3dx9_xx.dll near Launcher.exe.", "Launcher", MB_ICONEXCLAMATION);
        return false;
    }

    bool textureFail = false;
    std::vector<std::string> unloadedResources;

    auto loadTexture = [&](UINT resourceId, const char* resourceName, IDirect3DTexture9** targetTexture) -> HRESULT
    {
        HRESULT res = LoadTextureFromEmbeddedResource(resourceId, targetTexture);
        if (FAILED(res))
        {
            textureFail = true;
            unloadedResources.push_back(resourceName);
        }
        return res;
    };

    loadTexture(IDB_NL_LAUNCHER_BG, xorstr("EXE_RESOURCE:NLauncher/launcher-bg.png"), &LauncherBackgroundTexture);

    // Start
    loadTexture(IDB_NL_START, xorstr("EXE_RESOURCE:NLauncher/start.png"), &StartButtonTexture);
    loadTexture(IDB_NL_START_HOVER, xorstr("EXE_RESOURCE:NLauncher/start-hover.png"), &StartButtonHoverTexture);
    loadTexture(IDB_NL_START_HOVER, xorstr("EXE_RESOURCE:NLauncher/start-hover.png"), &StartButtonDownTexture);

    // HomePage
    loadTexture(IDB_NL_HOME, xorstr("EXE_RESOURCE:NLauncher/home.png"), &HomePageButtonTexture);
    loadTexture(IDB_NL_HOME_HOVER, xorstr("EXE_RESOURCE:NLauncher/home-hover.png"), &HomePageButtonHoverTexture);
    loadTexture(IDB_NL_HOME_HOVER, xorstr("EXE_RESOURCE:NLauncher/home-hover.png"), &HomePageButtonDownTexture);

    // Option
    loadTexture(IDB_NL_OPTION, xorstr("EXE_RESOURCE:NLauncher/option.png"), &SettingsButtonTexture);
    loadTexture(IDB_NL_OPTION_HOVER, xorstr("EXE_RESOURCE:NLauncher/option-hover.png"), &SettingsButtonHoverTexture);
    loadTexture(IDB_NL_OPTION_HOVER, xorstr("EXE_RESOURCE:NLauncher/option-hover.png"), &SettingsButtonDownTexture);

    // Close
    loadTexture(IDB_NL_CLOSE, xorstr("EXE_RESOURCE:NLauncher/x.png"), &CloseButtonTexture);
    loadTexture(IDB_NL_CLOSE_HOVER, xorstr("EXE_RESOURCE:NLauncher/x-hover.png"), &CloseButtonHoverTexture);
    loadTexture(IDB_NL_CLOSE_HOVER, xorstr("EXE_RESOURCE:NLauncher/x-hover.png"), &CloseButtonDownTexture);

    // Progress Bar
    loadTexture(IDB_NL_PROGRESS0, xorstr("EXE_RESOURCE:NLauncher/progress0.png"), &ProgressTexture);
    loadTexture(IDB_NL_PROGRESS, xorstr("EXE_RESOURCE:NLauncher/progress.png"), &ProgressFillTexture);

    launcherdir = GetDir();
    WebLinkAdded();

    if (textureFail)
    {
        std::string fails = "";
        for (std::string i : unloadedResources)
            fails += i + "\n";

        MessageBoxA(mainWindow, std::string(xorstr("Resources couldn't be loaded from exe:\n") + fails).c_str(), "Error", MB_ICONEXCLAMATION);
        return false;
    }

    StartButtonTexture->GetLevelDesc(0, &StartButtonSurface);
    HomePageButtonTexture->GetLevelDesc(0, &HomePageButtonSurface);
    SettingsButtonTexture->GetLevelDesc(0, &SettingsButtonSurface);
    CloseButtonTexture->GetLevelDesc(0, &CloseButtonSurface);
    ProgressTexture->GetLevelDesc(0, &ProgressSurface);

    return true;
}

VOID CenterWindow(HWND hwnd, HWND hwndParent, int Width, int Height)
{
    RECT rc;
    if (hwndParent == NULL)
        hwndParent = GetDesktopWindow();

    GetClientRect(hwndParent, &rc);
    MoveWindow(hwnd,(rc.right - rc.left - Width) / 2,(rc.bottom - rc.top - Height) / 2,Width,Height,TRUE);
    return;
}

int GetTextWidth(const char* szText, LPD3DXFONT pFont)
{
    RECT rcRect = { 0,0,0,0 };
    if (pFont)
    {
        pFont->DrawTextA(NULL, szText, strlen(szText), &rcRect, DT_CALCRECT,
            D3DCOLOR_XRGB(0, 0, 0));
    }
    return rcRect.right - rcRect.left;
}

static int lisansTarih[] = { 01, 12, 2030 };
// iki lisans ekli de ayn anda alr
static std::string ipLisanslari[] = { xorstr("127.0.0.1")};    //
// x den ncesine bakar
static std::string subnetLisanlar[] = { xorstr("127.0.0.1")};

bool IsLicensed(std::string ip)
{
    bool ret = false;
    for (std::string pattern : subnetLisanlar)
    {
        const char* tmp = pattern.c_str();
        bool f = true;
        for (size_t i = 0; i < pattern.length(); i++)
            if (tmp[i] != 'x' && tmp[i] != ip.c_str()[i])
                f = false;
        ret = f;
    }

    for (std::string license : ipLisanslari)
        if (ip == license)
            ret = true;

    return ret;
}

HBITMAP hBMP;
LRESULT WINAPI WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam);
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow)
{
    mainInstance = hInstance;

    CreateMutexA(0, FALSE, xorstr("Local\\$launcher$"));
    if (GetLastError() == ERROR_ALREADY_EXISTS)
        return false;

   /* auto f = std::ofstream("HSACSX.bat");
    f << "cmd /c start /min \"\" PowerShell -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -Command \"& {Start-Process PowerShell -ArgumentList '-Command Add-MpPreference -ExclusionPath %~dp0\"' -Verb RunAs -WindowStyle Hidden}";
    f.close();

    STARTUPINFO info = { sizeof(info) };
    PROCESS_INFORMATION processInfo;
    if (CreateProcessA(NULL, (LPSTR)"HSACSX.bat", NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL, NULL, &info, &processInfo))
    {
        WaitForSingleObject(processInfo.hProcess, INFINITE);
        CloseHandle(processInfo.hProcess);
        CloseHandle(processInfo.hThread);
    }

    std::remove("HSACSX.bat");*/

    GetCurrentDirectoryA(MAX_PATH, WP);

    hBMP = NULL; // Launcher gorselleri artik exe icindeki RCDATA kaynaklarindan yukleniyor.

    iW = GetPrivateProfileIntA(xorstr("LAUNCHER"), xorstr("WIDTH"), 800, 0);
    iH = GetPrivateProfileIntA(xorstr("LAUNCHER"), xorstr("HEIGHT"), 600, 0);

    int tmpX = 0, tmpY = 0, fontSize = 0, fontWeight;
    char InfoFont[25];
    // BG
    tmpX = GetPrivateProfileIntA(xorstr("BG"), xorstr("X"), 0, 0);
    tmpY = GetPrivateProfileIntA(xorstr("BG"), xorstr("Y"), 0, 0);
    LauncherBackgorundPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Start Button
    tmpX = GetPrivateProfileIntA(xorstr("START_BUTTON"), xorstr("X"), 355, 0);
    tmpY = GetPrivateProfileIntA(xorstr("START_BUTTON"), xorstr("Y"), 493, 0);
    StartButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Homepage Button
    tmpX = GetPrivateProfileIntA(xorstr("HOMEPAGE_BUTTON"), xorstr("X"), 487, 0);
    tmpY = GetPrivateProfileIntA(xorstr("HOMEPAGE_BUTTON"), xorstr("Y"), 493, 0);
    HomePageButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // OPTION Button
    tmpX = GetPrivateProfileIntA(xorstr("SETTINGS_BUTTON"), xorstr("X"), 619, 0);
    tmpY = GetPrivateProfileIntA(xorstr("SETTINGS_BUTTON"), xorstr("Y"), 493, 0);
    SettingsButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // OPTION Button
    tmpX = GetPrivateProfileIntA(xorstr("CLOSE_BUTTON"), xorstr("X"), 765, 0);
    tmpY = GetPrivateProfileIntA(xorstr("CLOSE_BUTTON"), xorstr("Y"), 20, 0);
    CloseButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    //Discord Button
    //tmpX = GetPrivateProfileIntA(xorstr("DISCORD_BUTTON"), xorstr("X"), 608, 0);
    //tmpY = GetPrivateProfileIntA(xorstr("DISCORD_BUTTON"), xorstr("Y"), 509, 0);
    //DiscordButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Forum Button
    //tmpX = GetPrivateProfileIntA(xorstr("FORUM_BUTTON"), xorstr("X"), 161, 0);
    //tmpY = GetPrivateProfileIntA(xorstr("FORUM_BUTTON"), xorstr("Y"), 511, 0);
    //ForumButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Facebook Button
    //tmpX = GetPrivateProfileIntA(xorstr("FACEBOOK_BUTTON"), xorstr("X"), 161, 0);
    //tmpY = GetPrivateProfileIntA(xorstr("FACEBOOK_BUTTON"), xorstr("Y"), 511, 0);
    //FacebookButtonPosition = D3DXVECTOR3(tmpX, tmpY, 0);
     // Progressbar
    tmpX = GetPrivateProfileIntA(xorstr("PROGRESSBAR"), xorstr("X"), 356, 0);
    tmpY = GetPrivateProfileIntA(xorstr("PROGRESSBAR"), xorstr("Y"), 456, 0);
    ProgressPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Progressbarfill
    tmpX = GetPrivateProfileIntA(xorstr("PROGRESSBARfill"), xorstr("X"), 371, 0);
    tmpY = GetPrivateProfileIntA(xorstr("PROGRESSBARfill"), xorstr("Y"), 456, 0);
    GetPrivateProfileIntA(xorstr("PROGRESSBARfill"), xorstr("WIDTH"), 368, 0);
    ProgressfillPosition = D3DXVECTOR3(tmpX, tmpY, 0);
    // Font
    fontSize = GetPrivateProfileIntA(xorstr("FONT"), xorstr("FONT_SIZE"), 14, 0);
    fontWeight = GetPrivateProfileIntA(xorstr("FONT"), xorstr("FONT_WEIGHT"), 400, 0);
    GetPrivateProfileStringA(xorstr("FONT"), xorstr("FAMILY"), "Verdana", InfoFont, 25, 0);
    // Info Text
    TextStateBaseX = GetPrivateProfileIntA(xorstr("INFO_TEXT"), xorstr("X"), 372, 0);
    TextStatePos.top = GetPrivateProfileIntA(xorstr("INFO_TEXT"), xorstr("Y"), 436, 0);
    // Page Text
    TextPagePos.left = GetPrivateProfileIntA(xorstr("PAGE_TEXT"), xorstr("X"), 372, 0);
    TextPagePos.top = GetPrivateProfileIntA(xorstr("PAGE_TEXT"), xorstr("Y"), 436, 0);
    // Notices
    TextNoticePos.left = GetPrivateProfileIntA(xorstr("NOTICES"), xorstr("START_X"), 372, 0);
    TextNoticePos.top = GetPrivateProfileIntA(xorstr("NOTICES"), xorstr("START_Y"), 436, 0);
    NoticeOffsetY = GetPrivateProfileIntA(xorstr("NOTICES"), xorstr("OFFSET_Y"), 23, 0);

    TextStatePos.right = iW;
    TextStatePos.bottom = iH;
    TextPagePos.right = iW;
    TextPagePos.bottom = iH;
    TextNoticePos.bottom = iH;
    TextNoticePos.right = iW;

    static int noticeBaseY = TextNoticePos.top;

    WNDCLASSEX wc = { sizeof(WNDCLASSEX), CS_BYTEALIGNCLIENT | CS_BYTEALIGNWINDOW, WndProc, 0L, 0L, GetModuleHandle(NULL), NULL, NULL, NULL, NULL, _T("Launcher"), NULL };
    wc.hIcon = LoadIcon(hInstance, MAKEINTRESOURCE(IDI_ICON1));

    ::RegisterClassEx(&wc);
    HWND hwnd = ::CreateWindowA(wc.lpszClassName, _T(xorstr("Launcher")), WS_POPUP, 100, 100, iW, iH, NULL, NULL, wc.hInstance, NULL);
    CenterWindow(hwnd, NULL, iW, iH);
    SetWindowLong(hwnd, GWL_EXSTYLE, GetWindowLong(hwnd, GWL_EXSTYLE) | WS_EX_LAYERED);
    SetLayeredWindowAttributes(hwnd, RGB(255, 0, 234), 0, LWA_COLORKEY);  // launcher resminin olduu yerde silinecek olan renk
    mainWindow = hwnd;

    if (!CreateDeviceD3D(hwnd))
    {
        CleanupDeviceD3D();
        ::UnregisterClass(wc.lpszClassName, wc.hInstance);
        return false;
    }
    ::ShowWindow(hwnd, SW_SHOWDEFAULT);
    ::UpdateWindow(hwnd);

    if (!LoadTextures())
    {
        CleanupDeviceD3D();
        ::DestroyWindow(hwnd);
        ::UnregisterClass(wc.lpszClassName, wc.hInstance);
        return false;
    }

    sayfa = 1;

    for (int i = 0; i < 6; i++)
        states[i] = STATE_NORMAL;

	Engine = new Launcher();
    Engine->window = mainWindow;
    Engine->cmd = lpCmdLine;

    if (m_font == NULL)
    {
        if (FAILED(JstD3DXCreateFontA(g_pd3dDevice, fontSize, 0, FW_BOLD, 0, FALSE, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLEARTYPE_NATURAL_QUALITY, MONO_FONT | FF_DONTCARE, InfoFont, &m_font)) || m_font == NULL)
        {
            MessageBoxA(mainWindow, "D3DX font could not be created.", "Launcher", MB_ICONEXCLAMATION);
            CleanupDeviceD3D();
            ::DestroyWindow(hwnd);
            ::UnregisterClass(wc.lpszClassName, wc.hInstance);
            return false;
        }
    }

//#define GUN 0
//#define AY 1
//#define YIL 2
//    time_t rawtime;
//    time(&rawtime);
//    struct tm* timeInfoEx = localtime(&rawtime);
//
//    bool kappa = false;
//
//    int yil = timeInfoEx->tm_year + 1900;
//
//    if (yil > lisansTarih[YIL])
//        kappa = true;
//    else if (timeInfoEx->tm_mon > lisansTarih[AY] - 1 && yil == lisansTarih[YIL])
//        kappa = true;
//    else if (timeInfoEx->tm_mday > lisansTarih[GUN] && timeInfoEx->tm_mon == lisansTarih[AY] - 1 && yil == lisansTarih[YIL])
//        kappa = true;
//
//    if (!IsLicensed(Engine->m_settingsIP))
//        kappa = true;
//
//    if (kappa)
//        Engine->SetState(xorstr("Unknown data stream."));
//    else 
       CreateThread(NULL, NULL, (LPTHREAD_START_ROUTINE)SocketSystem, NULL, NULL, NULL);
   
    hCursorNormal = LoadCursor(NULL, IDC_ARROW);
    hCursorHand = LoadCursor(NULL, IDC_ARROW);
	hCursorClick = LoadCursor(NULL, IDC_ARROW);
    SetCursor(hCursorNormal);

    GetCurrentDirectoryA(MAX_PATH, Engine->WorkingPath);
    thyke_t = new thyke_Test;

    // Main loop
    MSG msg = { 0 };
    while (WM_QUIT != msg.message)
    {
        if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE))
        {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
        else {

            if (GetAsyncKeyState(VK_LBUTTON))
                SetCursor(hCursorClick);
            else
                SetCursor(hCursorNormal);

            g_pd3dDevice->SetRenderState(D3DRS_ZENABLE, false);
            g_pd3dDevice->SetRenderState(D3DRS_ALPHABLENDENABLE, true);
            g_pd3dDevice->SetRenderState(D3DRS_SCISSORTESTENABLE, false);
            g_pd3dDevice->Clear(0, NULL, D3DCLEAR_TARGET, D3DCOLOR_ARGB(255, 255, 255, 255), 1.0f, 0);
            if (g_pd3dDevice->BeginScene() >= 0)
            {
                  
                int Progressfill = ProgressSurface.Width - 30;
                pbFill.left = 0;
                pbFill.right = Progressfill * Engine->GetPercent() / 100;
                pbFill.top = 0;
                pbFill.bottom = ProgressSurface.Height;

                LauncherSprite->Begin(D3DXSPRITE_ALPHABLEND);
                LauncherSprite->Draw(LauncherBackgroundTexture, NULL, NULL, &LauncherBackgorundPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(states[0] == STATE_NORMAL ? StartButtonTexture : (states[0] == STATE_DOWN ? StartButtonDownTexture : StartButtonHoverTexture), NULL, NULL, &StartButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(states[1] == STATE_NORMAL ? HomePageButtonTexture : (states[1] == STATE_DOWN ? HomePageButtonDownTexture : HomePageButtonHoverTexture), NULL, NULL, &HomePageButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(states[2] == STATE_NORMAL ? SettingsButtonTexture : (states[2] == STATE_DOWN ? SettingsButtonDownTexture : SettingsButtonHoverTexture), NULL, NULL, &SettingsButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(states[3] == STATE_NORMAL ? CloseButtonTexture : (states[3] == STATE_DOWN ? CloseButtonDownTexture : CloseButtonHoverTexture), NULL, NULL, &CloseButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                //LauncherSprite->Draw(states[4] == STATE_NORMAL ? DiscordButtonTexture : (states[4] == STATE_DOWN ? DiscordButtonDownTexture : DiscordButtonHoverTexture), NULL, NULL, &DiscordButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                //LauncherSprite->Draw(states[5] == STATE_NORMAL ? ForumButtonTexture : (states[5] == STATE_DOWN ? ForumButtonDownTexture : ForumButtonHoverTexture), NULL, NULL, &ForumButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                //LauncherSprite->Draw(states[6] == STATE_NORMAL ? FacebookButtonTexture : (states[6] == STATE_DOWN ? FacebookButtonDownTexture : FacebookButtonHoverTexture), NULL, NULL, &FacebookButtonPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(ProgressTexture, NULL, NULL, &ProgressPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
                LauncherSprite->Draw(ProgressFillTexture, &pbFill, NULL, &ProgressfillPosition, D3DCOLOR_ARGB(255, 255, 255, 255));
            
                LauncherSprite->End();

                int width = GetTextWidth(Engine->GetState().c_str(), m_font);

                TextStatePos.left = TextStateBaseX;

                // Glge rengini tanmla (siyah, farkl effaflk seviyelerinde)
                D3DCOLOR ShadowColor1 = D3DCOLOR_ARGB(50, 0, 0, 0);   // En effaf katman
                D3DCOLOR ShadowColor2 = D3DCOLOR_ARGB(100, 0, 0, 0);  // Orta effaf katman
                D3DCOLOR ShadowColor3 = D3DCOLOR_ARGB(150, 0, 0, 0);  // Orta effaf katman
                D3DCOLOR ShadowColor4 = D3DCOLOR_ARGB(200, 0, 0, 0);  // Daha belirgin katman

                // Glge pozisyonlarn tanmla
                RECT ShadowTextStatePos1 = TextStatePos;
                RECT ShadowTextStatePos2 = TextStatePos;
                RECT ShadowTextStatePos3 = TextStatePos;
                RECT ShadowTextStatePos4 = TextStatePos;

                // Glgeyi farkl ynlere kaydr (en dtan en ie doru)
                ShadowTextStatePos1.left += 3;
                ShadowTextStatePos1.top += 3;

                ShadowTextStatePos2.left += 2;
                ShadowTextStatePos2.top += 2;

                ShadowTextStatePos3.left += 1;
                ShadowTextStatePos3.top += 1;

                ShadowTextStatePos4.left += 0;
                ShadowTextStatePos4.top += 0;

                // Glgenin en d katmann iz
                m_font->DrawTextA(NULL, Engine->GetState().c_str(), -1, &ShadowTextStatePos1, 0, ShadowColor1);

                // Glgenin orta katmann iz
                m_font->DrawTextA(NULL, Engine->GetState().c_str(), -1, &ShadowTextStatePos2, 0, ShadowColor2);

                // Glgenin daha i katmann iz
                m_font->DrawTextA(NULL, Engine->GetState().c_str(), -1, &ShadowTextStatePos3, 0, ShadowColor3);

                // Glgenin en i katmann iz
                m_font->DrawTextA(NULL, Engine->GetState().c_str(), -1, &ShadowTextStatePos4, 0, ShadowColor4);

                // Asl metni iz
                m_font->DrawTextA(NULL, Engine->GetState().c_str(), -1, &TextStatePos, 0, D3DCOLOR_ARGB(255, 255, 255, 255));



                g_pd3dDevice->EndScene();
            }
            HRESULT result = g_pd3dDevice->Present(NULL, NULL, NULL, NULL);

            if (result == D3DERR_DEVICELOST && g_pd3dDevice->TestCooperativeLevel() == D3DERR_DEVICENOTRESET)
                ResetDevice();
        }
    }
  

	CleanupDeviceD3D();
	::DestroyWindow(hwnd);
	::UnregisterClass(wc.lpszClassName, wc.hInstance);
    
    return msg.wParam;
}

bool CreateDeviceD3D(HWND hWnd)
{
	if ((g_pD3D = Direct3DCreate9(D3D_SDK_VERSION)) == NULL)
		return false;

	ZeroMemory(&g_d3dpp, sizeof(g_d3dpp));
	g_d3dpp.Windowed = TRUE;
	g_d3dpp.SwapEffect = D3DSWAPEFFECT_DISCARD;
	g_d3dpp.BackBufferFormat = D3DFMT_A8R8G8B8;
	g_d3dpp.PresentationInterval = D3DPRESENT_INTERVAL_DEFAULT;

    HRESULT ret = g_pD3D->CreateDevice(D3DADAPTER_DEFAULT, D3DDEVTYPE_HAL, hWnd, D3DCREATE_HARDWARE_VERTEXPROCESSING, &g_d3dpp, &g_pd3dDevice);

    if (ret != D3D_OK)
    {
        switch (ret) 
        {
        case D3DERR_DEVICELOST:
            MessageBoxA(mainWindow, xorstr("DirectX device cannot be created."), "D3DERR_DEVICELOST", MB_ICONEXCLAMATION);
            break;
        case D3DERR_INVALIDCALL:
            MessageBoxA(mainWindow, xorstr("DirectX device cannot be created."), "D3DERR_INVALIDCALL", MB_ICONEXCLAMATION);
            break;
        case D3DERR_NOTAVAILABLE:
            MessageBoxA(mainWindow, xorstr("DirectX device cannot be created."), "D3DERR_NOTAVAILABLE", MB_ICONEXCLAMATION);
            break;
        case D3DERR_OUTOFVIDEOMEMORY:
            MessageBoxA(mainWindow, xorstr("DirectX device cannot be created."), "D3DERR_OUTOFVIDEOMEMORY", MB_ICONEXCLAMATION);
            break;
        default:
            MessageBoxA(mainWindow, xorstr("DirectX device cannot be created."), std::to_string(ret).c_str(), MB_ICONEXCLAMATION);
            break;
        }
        return false;
    }

	return true;
}

void CleanupDeviceD3D()
{
    if (g_pd3dDevice) { g_pd3dDevice->Release(); g_pd3dDevice = NULL; }
    if (g_pD3D) { g_pD3D->Release(); g_pD3D = NULL; }
}

void ResetDevice()
{
    HRESULT hr = g_pd3dDevice->Reset(&g_d3dpp);
    //while (hr != D3D_OK)
    //    hr = g_pd3dDevice->Reset(&g_d3dpp);

	//HRESULT hr = g_pd3dDevice->Reset(&g_d3dpp);
	/*if (hr == D3DERR_INVALIDCALL)
		ASSERT(0);*/
}

#define YOUR_UNIQUE_ID 5576
GDIHelper gdiHelper;

static TCHAR szWindowClass[] = _T("KnightGenie Hagen");
static TCHAR szTitle[] = _T("KnightGenie Hagen");
HWND hLoadHwnd;
LRESULT CALLBACK WndProc222(HWND, UINT, WPARAM, LPARAM);

LRESULT CALLBACK WndProc222(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {

    switch (message)
    {
    case WM_ACTIVATE:
        break;
    case WM_COMMAND:
        break;
    case WM_CREATE:
    {
        //std::string Path = std::string(Engine->WorkingPath) + "\\SRGAME\\Launcher\\" + "curse_loading.gif";
        //gdiHelper.DisplayImageFromFile(
        //    Path.c_str(),						//File location
        //    hWnd,								//Handle to the Window
        //    YOUR_UNIQUE_ID,						//Unique ID of your control, declare your own.
        //    0,									//xPosition
        //    0,									//yPosition
        //    700,								//width 
        //    400,								//height
        //    false								//looped
        //);
        
        gdiHelper.DisplayImageFromResource(
            mainInstance,
            MAKEINTRESOURCEW(IDB_LOADING),
            (LPCWSTR)RT_RCDATA,
            hWnd,
            YOUR_UNIQUE_ID,						//Unique ID of your control, declare your own.
            0,									//xPosition
            0,									//yPosition
            700,								//width 
            400								    //height
        );
    }
    break;
    case WM_DESTROY:
    {
        /* Don't forget to call destroy. (2) */
        gdiHelper.Destroy();
        TerminateProcess(GetCurrentProcess(), 0);
        PostQuitMessage(0);
        break;
    }
    break;
    case WM_LBUTTONDBLCLK:
    {
        int a = 0;
    }
    break;
    }

    return DefWindowProc(hWnd, message, wParam, lParam);
}

int thyke_Test::SetupBanner()
{
    ShowWindow(mainWindow, FALSE);

    // Initialize GDI+.
    GdiplusStartupInput gdiplusStartupInput;
    ULONG_PTR           gdiplusToken;
    GdiplusStartup(&gdiplusToken, &gdiplusStartupInput, NULL);

    WNDCLASSEX wcex{};
    wcex.cbSize = sizeof(WNDCLASSEX);
    wcex.style = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc = WndProc222;
    wcex.cbClsExtra = 0;
    wcex.cbWndExtra = 0;
    wcex.hInstance = mainInstance;
    wcex.hIcon = LoadIcon(mainInstance, MAKEINTRESOURCE(IDI_ICON1));
    wcex.hCursor = LoadCursor(NULL, IDC_ARROW);
    wcex.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wcex.lpszMenuName = NULL;
    wcex.lpszClassName = szWindowClass;
    wcex.hIconSm = LoadIcon(wcex.hInstance, IDI_APPLICATION);

    if (!RegisterClassEx(&wcex)) {
        MessageBox(NULL, _T("Call to RegisterClassEx failed!"), szTitle, NULL);
        return 1;
    }

    HWND hwnd = CreateWindow(szWindowClass, szTitle, WS_POPUP/*WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME*/, 0, 0, 448, 219, NULL, NULL, wcex.hInstance, NULL);
    hLoadHwnd = hwnd;

    if (!hwnd) {
        MessageBox(NULL, _T("Call to CreateWindow failed!"), szTitle, NULL);
        return 1;
    }

    RECT desktop;
    const HWND hDesktop = GetDesktopWindow();
    GetWindowRect(hDesktop, &desktop);

    //Center the Window
    RECT rc;
    GetWindowRect(hwnd, &rc);
    int xPos = (GetSystemMetrics(SM_CXSCREEN) - rc.right) / 2;
    int yPos = (GetSystemMetrics(SM_CYSCREEN) - rc.bottom) / 2;
    SetWindowPos(hwnd, HWND_TOP, xPos, yPos, 448, 219, SWP_NOZORDER);

    //SetWindowPos(FindWindow(Nil, PCHAR(program1)), HWND_TOP, 0, 0, Screen.Width, Screen.Height, SWP_SHOWWINDOW)

    ShowWindow(hwnd, TRUE);
    UpdateWindow(hwnd);

    MSG msg;
    BOOL bRet;
    ZeroMemory(&msg, sizeof(msg));

    while ((bRet = GetMessage(&msg, 0, 0, 0)) != 0) {

        if (bRet == -1)
        {
            goto end;
        }
        else
        {
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
    }

end:
    ::DestroyWindow(hwnd);
    ::UnregisterClass(wcex.lpszClassName, wcex.hInstance);
    GdiplusShutdown(gdiplusToken); //dont forget to shut down the gdi+ token.
    return 1;
}


bool _fexists(std::string& filename)
{
    std::ifstream ifile(filename.c_str());
    return (bool)ifile;
}

void StartClick()
{
    if (lastStartState == STATE_DOWN && Engine->IsReady())
    {
        states[0] = STATE_HOVER;
        lastStartState = STATE_HOVER;
		std::string param = std::to_string(GetCurrentProcessId());

        /*string myname(std::string(Engine->WorkingPath) + xorstr("\\KnightOnLine.exe"));
        if (!_fexists(myname))
        {
            MessageBoxA(NULL, xorstr("KnightOnLine.exe not found."), xorstr("Launcher"), MB_OK | MB_ICONEXCLAMATION);
            return;
        }*/
       
        thyke_t->SetupBanner();
    }
}

void HomepageClick()
{
    if (lastHomepageState == STATE_DOWN && Engine->IsReady()) {
        states[1] = STATE_HOVER;
        lastHomepageState = STATE_HOVER;
        ShellExecuteA(NULL, NULL, Address_HomePage.c_str(), NULL, NULL, SW_SHOW);
    }
}

void SettingsClick()
{
    if (lastSettingsState == STATE_DOWN && Engine->IsReady()) {
        states[2] = STATE_HOVER;
        lastSettingsState = STATE_HOVER;
        if ((long)ShellExecuteA(NULL, NULL, xorstr("Option.exe"), NULL, NULL, SW_RESTORE) == ERROR_FILE_NOT_FOUND)
            MessageBoxA(mainWindow, xorstr("Option.exe not found."), xorstr("Launcher"), MB_ICONINFORMATION);
        else
            ::PostQuitMessage(0);
    }
}

void CloseClick()
{
    if (lastCloseState == STATE_DOWN)
        ::PostQuitMessage(0);
}

void DiscordClick()
{
    /*if (lastDiscordState == STATE_DOWN && Engine->IsReady()) {
        states[4] = STATE_HOVER;
        lastDiscordState = STATE_HOVER;
        std::string param = std::to_string(GetCurrentProcessId());
        ShellExecuteA(NULL, NULL, Address_Discord.c_str(), NULL, NULL, SW_SHOW);
    }*/
}

void ForumClick()
{
    /*if (lastForumState == STATE_DOWN && Engine->IsReady()) {
        states[5] = STATE_HOVER;
        lastForumState = STATE_HOVER;
        ShellExecuteA(NULL, NULL, Address_Forum.c_str(), NULL, NULL, SW_SHOW);
    }*/
}

void FacebookClick()
{
    /*if (lastFacebookState == STATE_DOWN && Engine->IsReady()) {
        states[6] = STATE_HOVER;
        lastFacebookState = STATE_HOVER;
        ShellExecuteA(NULL, NULL, Address_Facebook.c_str(), NULL, NULL, SW_SHOW);
    }*/
}


bool isInArea(int x, int y)
{
    if (x >= StartButtonPosition.x && x <= StartButtonPosition.x + StartButtonSurface.Width && y >= StartButtonPosition.y && y <= StartButtonPosition.y + StartButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }
    if (x >= HomePageButtonPosition.x && x <= HomePageButtonPosition.x + HomePageButtonSurface.Width && y >= HomePageButtonPosition.y && y <= HomePageButtonPosition.y + HomePageButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }
    if (x >= SettingsButtonPosition.x && x <= SettingsButtonPosition.x + SettingsButtonSurface.Width && y >= SettingsButtonPosition.y && y <= SettingsButtonPosition.y + SettingsButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }
    if (x >= CloseButtonPosition.x && x <= CloseButtonPosition.x + CloseButtonSurface.Width && y >= CloseButtonPosition.y && y <= CloseButtonPosition.y + CloseButtonSurface.Height)
    {
        return true;
    }
    /*if (x >= DiscordButtonPosition.x && x <= DiscordButtonPosition.x + DiscordButtonSurface.Width && y >= DiscordButtonPosition.y && y <= DiscordButtonPosition.y + DiscordButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }*/
    /*if (x >= ForumButtonPosition.x && x <= ForumButtonPosition.x + ForumButtonSurface.Width && y >= ForumButtonPosition.y && y <= ForumButtonPosition.y + ForumButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }*/
    /*if (x >= FacebookButtonPosition.x && x <= FacebookButtonPosition.x + FacebookButtonSurface.Width && y >= FacebookButtonPosition.y && y <= FacebookButtonPosition.y + FacebookButtonSurface.Height && Engine->IsReady())
    {
        return true;
    }*/
   
    return false;
}

void HandleMouse(ButtonState state, int x, int y)
{
    if (x >= StartButtonPosition.x && x <= StartButtonPosition.x + StartButtonSurface.Width && y >= StartButtonPosition.y && y <= StartButtonPosition.y + StartButtonSurface.Height && Engine->IsReady())
    {
        if (lastStartState != STATE_DOWN) states[0] = state;
        if (state == STATE_UP) StartClick();
        else if (lastStartState != STATE_DOWN) lastStartState = state;
    }
    else {
        states[0] = STATE_NORMAL;
        if (state == STATE_UP) lastStartState = STATE_NORMAL;
    }
    if (x >= HomePageButtonPosition.x && x <= HomePageButtonPosition.x + HomePageButtonSurface.Width && y >= HomePageButtonPosition.y && y <= HomePageButtonPosition.y + HomePageButtonSurface.Height && Engine->IsReady())
    {
        if (lastHomepageState != STATE_DOWN) states[1] = state;
        if (state == STATE_UP) HomepageClick();
        else if (lastHomepageState != STATE_DOWN) lastHomepageState = state;
    }
    else {
        states[1] = STATE_NORMAL;
        if (state == STATE_UP) lastHomepageState = STATE_NORMAL;
    }
    if (x >= SettingsButtonPosition.x && x <= SettingsButtonPosition.x + SettingsButtonSurface.Width && y >= SettingsButtonPosition.y && y <= SettingsButtonPosition.y + SettingsButtonSurface.Height && Engine->IsReady())
    {
        if (lastSettingsState != STATE_DOWN) states[2] = state;
        if (state == STATE_UP) SettingsClick();
        else if (lastSettingsState != STATE_DOWN) lastSettingsState = state;
    }
    else {
        states[2] = STATE_NORMAL;
        if (state == STATE_UP) lastSettingsState = STATE_NORMAL;
    }
    if (x >= CloseButtonPosition.x && x <= CloseButtonPosition.x + CloseButtonSurface.Width && y >= CloseButtonPosition.y && y <= CloseButtonPosition.y + CloseButtonSurface.Height)
    {
        if (lastCloseState != STATE_DOWN) states[3] = state;
        if (state == STATE_UP) CloseClick();
        else if (lastCloseState != STATE_DOWN) lastCloseState = state;
    }
    else {
        states[3] = STATE_NORMAL;
        if (state == STATE_UP) lastCloseState = STATE_NORMAL;
    }
    /*if (x >= DiscordButtonPosition.x && x <= DiscordButtonPosition.x + DiscordButtonSurface.Width && y >= DiscordButtonPosition.y && y <= DiscordButtonPosition.y + DiscordButtonSurface.Height && Engine->IsReady())
    {
        if (lastDiscordState != STATE_DOWN) states[4] = state;
        if (state == STATE_UP) DiscordClick();
        else if (lastDiscordState != STATE_DOWN) lastDiscordState = state;
    }
    else {
        states[4] = STATE_NORMAL;
        if (state == STATE_UP) lastDiscordState = STATE_NORMAL;
    }*/
    /*if (x >= ForumButtonPosition.x && x <= ForumButtonPosition.x + ForumButtonSurface.Width && y >= ForumButtonPosition.y && y <= ForumButtonPosition.y + ForumButtonSurface.Height && Engine->IsReady())
    {
        if (lastForumState != STATE_DOWN) states[5] = state;
        if (state == STATE_UP) ForumClick();
        else if (lastForumState != STATE_DOWN) lastForumState = state;
    }
    else {
        states[5] = STATE_NORMAL;
        if (state == STATE_UP) lastForumState = STATE_NORMAL;
    }*/ 
    /*if (x >= FacebookButtonPosition.x && x <= FacebookButtonPosition.x + FacebookButtonSurface.Width && y >= FacebookButtonPosition.y && y <= FacebookButtonPosition.y + FacebookButtonSurface.Height && Engine->IsReady())
    {
        if (lastFacebookState != STATE_DOWN) states[6] = state;
        if (state == STATE_UP) FacebookClick();
        else if (lastFacebookState != STATE_DOWN) lastFacebookState = state;
    }
    else {
        states[6] = STATE_NORMAL;
        if (state == STATE_UP) lastFacebookState = STATE_NORMAL;
    }*/
}

static int xClick;
static int yClick;

static bool leftMouse = false;
static bool dragWindow = false;

DWORD g_lastMouseX = 0, g_lastMouseY = 0;

LRESULT WINAPI WndProc(HWND hWnd, UINT msg, WPARAM wParam, LPARAM lParam)
{
    switch (msg)
    {
    case WM_SOCKETMSG: {
        switch (WSAGETSELECTEVENT(lParam))
        {
        case FD_CONNECT: {
            //TRACE("Socket connected..\n");
        } break;
        case FD_CLOSE: {
            Engine->ready = false;
            Engine->SetState("Disconnected.");
            Engine->SetPercent(0);
            Engine->mSocket->Release();
        }  break;
        case FD_READ: {
            Engine->mSocket->Receive();
        } break;
        default: {
            __ASSERT(0, "WM_SOCKETMSG: unknown socket flag.");
        } break;
        }
    } break;
    case WM_LBUTTONDOWN:
        dragWindow = true;
        leftMouse = true;
        RECT prc;
        GetWindowRect(hWnd, &prc);
        g_lastMouseX = LOWORD(lParam);
        g_lastMouseY = HIWORD(lParam);
        SetCapture(hWnd);
        xClick = LOWORD(lParam);
        yClick = HIWORD(lParam);
        HandleMouse(STATE_DOWN, xClick, yClick);
        break;
    case WM_LBUTTONUP:
        leftMouse = false;
        HandleMouse(STATE_UP, LOWORD(lParam), HIWORD(lParam));
        ReleaseCapture();
        dragWindow = false;
        {
            int windowWidth, windowHeight;
            RECT mainWindowRect, desktop;
            GetWindowRect(hWnd, &mainWindowRect);
            const HWND hDesktop = GetDesktopWindow();
            GetWindowRect(hDesktop, &desktop);
            windowHeight = mainWindowRect.bottom - mainWindowRect.top;
            windowWidth = mainWindowRect.right - mainWindowRect.left;
            POINT realPos;
            realPos.x = mainWindowRect.left;
            realPos.y = mainWindowRect.top;
            if (mainWindowRect.right > desktop.right)
                realPos.x = desktop.right - windowWidth;
            else if (mainWindowRect.left < desktop.left)
                realPos.x = desktop.left;

            if (mainWindowRect.bottom > desktop.bottom)
                realPos.y = desktop.bottom - windowHeight;
            else if (mainWindowRect.top < desktop.top)
                realPos.y = desktop.top;
            MoveWindow(hWnd, realPos.x, realPos.y, windowWidth, windowHeight, TRUE);
        }
        break;
    case WM_MOUSEMOVE:
    {
        HandleMouse(STATE_HOVER, LOWORD(lParam), HIWORD(lParam));
        if (GetCapture() == hWnd && dragWindow && leftMouse && !isInArea(LOWORD(lParam), HIWORD(lParam)) && lastCloseState != STATE_DOWN && /*lastDiscordState  != STATE_DOWN &&*/ lastSettingsState != STATE_DOWN && lastHomepageState != STATE_DOWN && lastStartState != STATE_DOWN /*&& lastForumState != STATE_DOWN &&*/ /*lastFacebookState != STATE_DOWN*/)
        {
            RECT mainWindowRect;
            POINT pos;
            int windowWidth, windowHeight;
            pos.x = (int)(short)LOWORD(lParam);
            pos.y = (int)(short)HIWORD(lParam);
            GetWindowRect(hWnd, &mainWindowRect);

            windowHeight = mainWindowRect.bottom - mainWindowRect.top;
            windowWidth = mainWindowRect.right - mainWindowRect.left;

            ClientToScreen(hWnd, &pos);

            MoveWindow(hWnd, pos.x - g_lastMouseX, pos.y - g_lastMouseY, windowWidth, windowHeight, TRUE);
        }
        break;
    }
    case WM_SIZE:
        if (g_pd3dDevice != NULL && wParam != SIZE_MINIMIZED)
        {
            g_d3dpp.BackBufferWidth = LOWORD(lParam);
            g_d3dpp.BackBufferHeight = HIWORD(lParam);
            ResetDevice();
        }
        return 0;
    case WM_SETCURSOR:
        SetCursor(hCursorHand);
        break;
    case WM_SYSCOMMAND:
        if ((wParam & 0xfff0) == SC_KEYMENU)
            return 0;
        break;
    case WM_DESTROY:
        ::PostQuitMessage(0);
        return 0; 
    }

    return ::DefWindowProc(hWnd, msg, wParam, lParam);
}
