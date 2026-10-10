// =============================================================================
// NTT private-server Launcher (splash + light cheat-process scan + start game)
//
//  - Shows a splash image (splash.gif animated / splash.png / splash.jpg) from
//    the launcher's own folder, centered, borderless.
//  - Scans running processes for well-known cheat/debug tools; if one is found,
//    warns and refuses to start (deterrent only -- real anti-cheat is server-side).
//  - Starts KnightOnLine.exe in the launcher's folder (no args, no injection:
//    XIGNCODE is already disabled in the exe and d3d9.dll auto-loads).
//
// Build (x86 Native Tools):
//   cl /nologo /std:c++17 /O2 /MT /EHsc launcher.cpp /Fe:Launcher.exe ^
//      /link /SUBSYSTEM:WINDOWS gdiplus.lib user32.lib gdi32.lib shell32.lib psapi.lib
// Put Launcher.exe + splash.gif (or .png/.jpg) next to KnightOnLine.exe.
// =============================================================================
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <gdiplus.h>
#include <psapi.h>
#include <string>
#include <vector>
#pragma comment(lib, "gdiplus.lib")

using namespace Gdiplus;

static const wchar_t* GAME_EXE = L"KnightOnLine.exe";
static const int      MIN_SPLASH_MS = 3000;   // minimum time the splash stays up

// --- known cheat/debug tools (lowercase substrings of the process exe name) ---
static const wchar_t* kBlacklist[] = {
    L"cheatengine", L"cheat engine", L"ollydbg", L"x64dbg", L"x32dbg", L"windbg",
    L"ida64", L"ida.exe", L"idaq", L"ghidra", L"artmoney", L"tsearch", L"wpepro",
    L"wireshark", L"fiddler", L"httpdebugger", L"processhacker", L"scylla",
    L"reclass", L"speeder", L"speedhack", L"cheatngine",
};

static Image*  g_img = nullptr;
static UINT    g_frameCount = 1, g_frame = 0;
static std::vector<UINT> g_frameDelayMs;   // per-frame delay for GIFs
static int     g_w = 480, g_h = 260;
static HWND    g_hwnd = nullptr;
static volatile bool g_launching = false;

static std::wstring ExeDir()
{
    wchar_t p[MAX_PATH]; GetModuleFileNameW(nullptr, p, MAX_PATH);
    std::wstring s(p); size_t i = s.find_last_of(L"\\/");
    return i == std::wstring::npos ? L"." : s.substr(0, i);
}

static Image* LoadSplash(const std::wstring& dir)
{
    const wchar_t* names[] = { L"splash.gif", L"splash.png", L"splash.jpg", L"launcher.gif", L"launcher.png" };
    for (auto n : names)
    {
        std::wstring path = dir + L"\\" + n;
        if (GetFileAttributesW(path.c_str()) == INVALID_FILE_ATTRIBUTES) continue;
        Image* im = new Image(path.c_str());
        if (im && im->GetLastStatus() == Ok) return im;
        delete im;
    }
    return nullptr;
}

static void ReadGifFrames(Image* im)
{
    UINT dims = im->GetFrameDimensionsCount();
    if (!dims) return;
    std::vector<GUID> ids(dims);
    im->GetFrameDimensionsList(ids.data(), dims);
    g_frameCount = im->GetFrameCount(&ids[0]);
    if (g_frameCount <= 1) { g_frameCount = 1; return; }
    UINT sz = im->GetPropertyItemSize(PropertyTagFrameDelay);
    if (sz)
    {
        std::vector<BYTE> buf(sz);
        auto* pi = (PropertyItem*)buf.data();
        if (im->GetPropertyItem(PropertyTagFrameDelay, sz, pi) == Ok)
        {
            long* d = (long*)pi->value;
            for (UINT i = 0; i < g_frameCount; i++)
            {
                UINT ms = (UINT)(d[i] * 10);          // stored in 1/100 s
                g_frameDelayMs.push_back(ms < 20 ? 100 : ms);
            }
        }
    }
    while (g_frameDelayMs.size() < g_frameCount) g_frameDelayMs.push_back(100);
}

static bool CheatToolRunning(std::wstring& found)
{
    DWORD pids[2048], cb = 0;
    if (!EnumProcesses(pids, sizeof(pids), &cb)) return false;
    for (DWORD i = 0; i < cb / sizeof(DWORD); i++)
    {
        HANDLE h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pids[i]);
        if (!h) continue;
        wchar_t name[MAX_PATH] = {};
        DWORD n = MAX_PATH;
        if (QueryFullProcessImageNameW(h, 0, name, &n))
        {
            std::wstring low = name;
            for (auto& c : low) c = (wchar_t)towlower(c);
            for (auto b : kBlacklist)
                if (low.find(b) != std::wstring::npos) { found = b; CloseHandle(h); return true; }
        }
        CloseHandle(h);
    }
    return false;
}

static void StartGame()
{
    std::wstring dir = ExeDir();
    std::wstring exe = dir + L"\\" + GAME_EXE;
    STARTUPINFOW si{ sizeof(si) };
    PROCESS_INFORMATION pi{};
    std::wstring cmd = L"\"" + exe + L"\"";
    if (CreateProcessW(nullptr, &cmd[0], nullptr, nullptr, FALSE, 0, nullptr, dir.c_str(), &si, &pi))
    {
        CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
    }
    else
    {
        MessageBoxW(nullptr, (GAME_EXE + std::wstring(L" bulunamadi.")).c_str(), L"Launcher", MB_ICONERROR);
    }
}

static DWORD WINAPI WorkThread(LPVOID)
{
    DWORD t0 = GetTickCount();
    std::wstring tool;
    if (CheatToolRunning(tool))
    {
        std::wstring m = L"Hile/debug programi acik: " + tool + L"\nLutfen kapatip tekrar deneyin.";
        MessageBoxW(nullptr, m.c_str(), L"Launcher", MB_ICONWARNING);
        if (g_hwnd) PostMessageW(g_hwnd, WM_CLOSE, 0, 0);
        ExitProcess(0);
    }
    DWORD spent = GetTickCount() - t0;
    if (spent < (DWORD)MIN_SPLASH_MS) Sleep(MIN_SPLASH_MS - spent);
    g_launching = true;
    StartGame();
    if (g_hwnd) PostMessageW(g_hwnd, WM_CLOSE, 0, 0);
    return 0;
}

static void Paint(HWND h)
{
    PAINTSTRUCT ps; HDC dc = BeginPaint(h, &ps);
    HDC mem = CreateCompatibleDC(dc);
    HBITMAP bmp = CreateCompatibleBitmap(dc, g_w, g_h);
    HBITMAP old = (HBITMAP)SelectObject(mem, bmp);
    Graphics g(mem);
    g.Clear(Color(255, 0, 0, 0));
    if (g_img) g.DrawImage(g_img, 0, 0, g_w, g_h);
    BitBlt(dc, 0, 0, g_w, g_h, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old); DeleteObject(bmp); DeleteDC(mem);
    EndPaint(h, &ps);
}

static LRESULT CALLBACK WndProc(HWND h, UINT m, WPARAM w, LPARAM l)
{
    switch (m)
    {
    case WM_TIMER:
        if (g_img && g_frameCount > 1)
        {
            g_frame = (g_frame + 1) % g_frameCount;
            GUID id = FrameDimensionTime;
            g_img->SelectActiveFrame(&id, g_frame);
            InvalidateRect(h, nullptr, FALSE);
            KillTimer(h, 1);
            SetTimer(h, 1, g_frameDelayMs.empty() ? 100 : g_frameDelayMs[g_frame], nullptr);
        }
        return 0;
    case WM_PAINT: Paint(h); return 0;
    case WM_DESTROY: PostQuitMessage(0); return 0;
    }
    return DefWindowProcW(h, m, w, l);
}

int WINAPI wWinMain(HINSTANCE hInst, HINSTANCE, LPWSTR, int)
{
    ULONG_PTR tok; GdiplusStartupInput gi; GdiplusStartup(&tok, &gi, nullptr);
    std::wstring dir = ExeDir();
    g_img = LoadSplash(dir);
    if (g_img)
    {
        g_w = (int)g_img->GetWidth(); g_h = (int)g_img->GetHeight();
        if (g_w < 64 || g_w > 1600) g_w = 480;
        if (g_h < 64 || g_h > 1000) g_h = 260;
        ReadGifFrames(g_img);
    }

    WNDCLASSW wc{}; wc.lpfnWndProc = WndProc; wc.hInstance = hInst;
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW); wc.lpszClassName = L"NTTLauncher";
    RegisterClassW(&wc);
    int sx = GetSystemMetrics(SM_CXSCREEN), sy = GetSystemMetrics(SM_CYSCREEN);
    g_hwnd = CreateWindowExW(WS_EX_TOPMOST, L"NTTLauncher", L"Launcher", WS_POPUP,
        (sx - g_w) / 2, (sy - g_h) / 2, g_w, g_h, nullptr, nullptr, hInst, nullptr);
    ShowWindow(g_hwnd, SW_SHOW); UpdateWindow(g_hwnd);
    if (g_img && g_frameCount > 1)
        SetTimer(g_hwnd, 1, g_frameDelayMs.empty() ? 100 : g_frameDelayMs[0], nullptr);

    CreateThread(nullptr, 0, WorkThread, nullptr, 0, nullptr);

    MSG msg;
    while (GetMessageW(&msg, nullptr, 0, 0)) { TranslateMessage(&msg); DispatchMessageW(&msg); }

    if (g_img) delete g_img;
    GdiplusShutdown(tok);
    return 0;
}
