#include "stdafx.h"
//#include "Engine.h"
#include "GDIHelper.h"
#include <iostream>
#include <sstream>
#include <Windows.h>
#include <shlwapi.h>
#include "Remap.h"

#pragma comment(lib, "shlwapi.lib")

extern GDIHelper gdiHelper;
extern HWND mainWindow;
HWND GDIHelper::staticControl = NULL;
HWND GDIHelper::MainHWNDControl = NULL;
Image* GDIHelper::m_pImage = NULL;
GUID* GDIHelper::m_pDimensionIDs = NULL;
UINT GDIHelper::m_FrameCount = 0;
PropertyItem* GDIHelper::m_pItem = NULL;
UINT GDIHelper::m_iCurrentFrame = 0;
UINT_PTR GDIHelper::unique_id = 0;
BOOL GDIHelper::m_bIsPlaying = FALSE;
BOOL GDIHelper::isPlayable = FALSE;
BOOL GDIHelper::isLooped = FALSE;
int GDIHelper::xPosition = 0;
int GDIHelper::yPosition = 0;
int GDIHelper::width = 0;
int GDIHelper::height = 0;
int GDIHelper::animation_duration = 0;

// Undocumented Native API tanımlamaları
typedef LONG(NTAPI* pfnNtSuspendProcess)(HANDLE ProcessHandle);
typedef LONG(NTAPI* pfnNtResumeProcess)(HANDLE ProcessHandle);

GDIHelper::GDIHelper() {}

void GDIHelper::Destroy() {
    if (m_pDimensionIDs) {
        delete[] m_pDimensionIDs;
    }
    if (m_pItem) {
        free(m_pItem);
    }
    if (m_pImage) {
        delete m_pImage;
    }
    m_bIsPlaying = FALSE;
    isPlayable = FALSE;
    RemoveWindowSubclass(staticControl, &StaticControlProc, unique_id);
}

std::string GetCurrentDirectory()
{
    char buffer[MAX_PATH];
    GetModuleFileName(NULL, buffer, MAX_PATH);
    PathRemoveFileSpec(buffer);
    return std::string(buffer);
}

extern HWND hLoadHwnd;

static bool PatchPusUrlInClient(HANDLE hProcess)
{
    static const uintptr_t kPusUrlAddress = 0x0103ACF8;
    static const char kOriginalUrl[] = "https://nttgame.com";
    static const char kStoreUrl[] = "http://84.247.183";

    char currentUrl[sizeof(kOriginalUrl)] = { 0 };

    for (int attempt = 0; attempt < 600; ++attempt)
    {
        SIZE_T bytesRead = 0;
        if (ReadProcessMemory(hProcess, reinterpret_cast<LPCVOID>(kPusUrlAddress), currentUrl, sizeof(currentUrl), &bytesRead)
            && bytesRead == sizeof(currentUrl)
            && memcmp(currentUrl, kOriginalUrl, sizeof(kOriginalUrl)) == 0)
        {
            char replacement[sizeof(kOriginalUrl)] = { 0 };
            memcpy(replacement, kStoreUrl, sizeof(kStoreUrl));

            DWORD oldProtect = 0;
            if (!VirtualProtectEx(hProcess, reinterpret_cast<LPVOID>(kPusUrlAddress), sizeof(replacement), PAGE_READWRITE, &oldProtect))
                return false;

            SIZE_T bytesWritten = 0;
            const BOOL written = WriteProcessMemory(hProcess, reinterpret_cast<LPVOID>(kPusUrlAddress), replacement, sizeof(replacement), &bytesWritten);

            DWORD ignoredProtect = 0;
            VirtualProtectEx(hProcess, reinterpret_cast<LPVOID>(kPusUrlAddress), sizeof(replacement), oldProtect, &ignoredProtect);

            return written && bytesWritten == sizeof(replacement);
        }
        Sleep(50);
    }
    return false;
}

void GDIHelper::run()
{
    if (m_bIsPlaying == TRUE) return;

    STARTUPINFO si = { sizeof(si) };
    PROCESS_INFORMATION pi = { 0 };

    std::string currentDir = "";
    std::string executablePath = "KnightOnline.exe";
    std::string parameters = "E03ED890-8E94-4B42-B1C5-3CDA401AA9C2";
    std::string commandLine = executablePath + " " + parameters;

    m_iCurrentFrame = 0;
    GUID Guid = FrameDimensionTime;
    m_pImage->SelectActiveFrame(&Guid, m_iCurrentFrame);
    ++m_iCurrentFrame;
    m_bIsPlaying = TRUE;
    animation_duration = ((UINT*)m_pItem->value)[m_iCurrentFrame] * 10;

    while (isPlayable) {
        std::this_thread::sleep_for(std::chrono::milliseconds(animation_duration));
        m_pImage->SelectActiveFrame(&Guid, m_iCurrentFrame);

        m_iCurrentFrame = (++m_iCurrentFrame) % m_FrameCount;
        InvalidateRect(staticControl, NULL, FALSE);
        UpdateWindow(staticControl);

        if (!isLooped && m_iCurrentFrame == 0)
        {
            ShowWindow(hLoadHwnd, SW_HIDE);
            ShowWindow(staticControl, SW_HIDE);
            gdiHelper.Destroy();

            currentDir = GetCurrentDirectory();

            // The game's own launch guard (NTT2625-d3d9proxy/launch_ticket.h, d3d9.dll) only
            // starts if a one-shot ticket was left by XIGNCODE\xldr_KnightOnline_NA_loader_win32.exe
            // running as OUR direct child -- run and wait for it first, same as the original
            // retail Launcher.exe did (see launch_ticket.h's header comment).
            {
                std::string xldrPath = currentDir + "\\XIGNCODE\\xldr_KnightOnline_NA_loader_win32.exe";
                STARTUPINFOA xldrSi = { sizeof(xldrSi) };
                PROCESS_INFORMATION xldrPi = { 0 };
                if (CreateProcessA(NULL, const_cast<LPSTR>(xldrPath.c_str()), NULL, NULL, FALSE, 0, NULL, NULL, &xldrSi, &xldrPi))
                {
                    WaitForSingleObject(xldrPi.hProcess, INFINITE);
                    CloseHandle(xldrPi.hThread);
                    CloseHandle(xldrPi.hProcess);
                }
            }

            // Start the game normally -- it loads d3d9.dll from its own folder (LoadLibraryA)
            // the same way every other JstKO client-side feature does; d3d9proxy.cpp then loads
            // JstKO.dll itself. No manual injection, no anti-cheat memory patching here.
            if (!CreateProcessA(NULL, const_cast<LPSTR>(commandLine.c_str()), NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi))
            {
                MessageBoxA(mainWindow, "KnightOnLine.exe bulunamadı.", "Launcher", MB_ICONINFORMATION);
                goto return_true;
            }

            // PUS URL Yaması uygulanıyor
            if (!PatchPusUrlInClient(pi.hProcess))
            {
                std::cout << "PUS URL uygulanamadı." << std::endl;
            }

            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        return_true:
            ::PostQuitMessage(0);
            TerminateProcess(GetCurrentProcess(), 0);
            ExitProcess(0);
            break;
        }
    }
}


bool GDIHelper::IsFileExist(string file_name) {
    struct stat buffer;
    return (stat(file_name.c_str(), &buffer) == 0);
}

void GDIHelper::GetImageFrame() {
    UINT count = m_pImage->GetFrameDimensionsCount();
    m_pDimensionIDs = new GUID[count];
    m_pImage->GetFrameDimensionsList(m_pDimensionIDs, count);
    m_FrameCount = m_pImage->GetFrameCount(&m_pDimensionIDs[0]);

    UINT TotalBuffer = m_pImage->GetPropertyItemSize(PropertyTagFrameDelay);
    m_pItem = (PropertyItem*)malloc(TotalBuffer);
    m_pImage->GetPropertyItem(PropertyTagFrameDelay, TotalBuffer, m_pItem);

    if (m_FrameCount > 1) {
        isPlayable = TRUE;
        std::thread t(run);
        t.detach();
    }
}

LRESULT CALLBACK GDIHelper::StaticControlProc(HWND hwnd, UINT uMsg, WPARAM wParam, LPARAM lParam, UINT_PTR uIdSubclass, DWORD_PTR dwRefData) {
    switch (uMsg)
    {
    case WM_PAINT:
    {
        PAINTSTRUCT ps;
        HDC hdc = BeginPaint(hwnd, &ps);
        Graphics g(hdc);
        g.DrawImage(m_pImage, 0, 0, width, height);
        EndPaint(hwnd, &ps);
        return TRUE;
    }
    break;
    }
    return DefSubclassProc(hwnd, uMsg, wParam, lParam);
}

void GDIHelper::SetImage(int uunique_id, int xxPosition, int yyPosition, int wwidth, int hheight, Image* image, HWND hwnd) {
    unique_id = uunique_id;
    xPosition = xxPosition;
    yPosition = yyPosition;
    width = wwidth;
    height = hheight;

    staticControl = CreateWindowEx(0, "STATIC", NULL, WS_CHILD | WS_VISIBLE | SS_OWNERDRAW, xPosition, yPosition, width, height, hwnd, NULL, NULL, NULL);
    SetWindowSubclass(staticControl, &StaticControlProc, unique_id, 0);
    m_pImage = image;
    GetImageFrame();
}

void GDIHelper::DisplayImageFromFile(string file_name, HWND hWnd, UINT_PTR uunique_id, int xxPosition, int yyPosition, int wwidth, int hheight, bool looped) {
    if (!IsFileExist(file_name)) return;
    isLooped = looped;
    std::wstring widestr = std::wstring(file_name.begin(), file_name.end());

    // C244/C4244 veri kaybı uyarısını önlemek için benzersiz ID int'e dönüştürülüyor
    SetImage(static_cast<int>(uunique_id), xxPosition, yyPosition, wwidth, hheight, Image::FromFile(widestr.c_str()), hWnd);
}

void GDIHelper::thyke_display(string file_name, HWND hWndKO, HWND& hWnd, UINT_PTR uunique_id, int xxPosition, int yyPosition, int wwidth, int hheight, bool looped) {
    if (!IsFileExist(file_name)) return;
    isLooped = looped;
    std::wstring widestr = std::wstring(file_name.begin(), file_name.end());

    unique_id = uunique_id;
    xPosition = xxPosition;
    yPosition = yyPosition;
    width = wwidth;
    height = hheight;

    staticControl = CreateWindowEx(0, "STATIC", NULL, WS_CHILD | WS_VISIBLE | SS_OWNERDRAW, xPosition, yPosition, width, height, hWndKO, NULL, NULL, NULL);
    hWnd = staticControl;
    SetWindowSubclass(staticControl, &StaticControlProc, unique_id, 0);
    m_pImage = Image::FromFile(widestr.c_str());
    GetImageFrame();
}

void GDIHelper::DisplayImageFromResource(HMODULE hMod, const wchar_t* resid, const wchar_t* restype, HWND hWnd, UINT_PTR uunique_id, int xxPosition, int yyPosition, int wwidth, int hheight) {
    IStream* pStream = nullptr;
    Gdiplus::Bitmap* pBmp = nullptr;
    HGLOBAL hGlobal = nullptr;
    HRSRC hrsrc = FindResourceW(GetModuleHandle(NULL), resid, restype);

    if (hrsrc) {
        DWORD dwResourceSize = SizeofResource(hMod, hrsrc);
        if (dwResourceSize > 0) {
            HGLOBAL hGlobalResource = LoadResource(hMod, hrsrc);
            if (hGlobalResource) {
                void* imagebytes = LockResource(hGlobalResource);
                hGlobal = ::GlobalAlloc(GHND, dwResourceSize);
                if (hGlobal) {
                    void* pBuffer = ::GlobalLock(hGlobal);
                    if (pBuffer) {
                        memcpy(pBuffer, imagebytes, dwResourceSize);
                        HRESULT hr = CreateStreamOnHGlobal(hGlobal, TRUE, &pStream);
                        if (SUCCEEDED(hr)) {
                            hGlobal = nullptr;
                            pBmp = new Gdiplus::Bitmap(pStream);
                        }
                    }
                }
            }
        }
    }

    if (pStream) pStream->Release();
    if (hGlobal) GlobalFree(hGlobal);

    // C244/C4244 veri kaybı uyarısını önlemek için benzersiz ID int'e dönüştürülüyor
    SetImage(static_cast<int>(uunique_id), xxPosition, yyPosition, wwidth, hheight, pBmp, hWnd);
}
