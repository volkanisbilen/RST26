// Offline render check of the Power Up Store skin: draws the panel with fake data
// and writes the raw BGRA frame buffer for conversion to PNG.
#include "pus_store.cpp"
#include <stdio.h>

void Log(const char* msg) { printf("%s\n", msg); }

int main(int argc, char** argv)
{
    InitializeCriticalSection(&g_sendLock);
    InitializeCriticalSection(&g_dataLock);
    strcpy_s(g_uiDir, "D:\\NTT2625-d3d9proxy\\pus_ui");

    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = DefWindowProcW; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"PusTest";
    RegisterClassExW(&wc);
    g_hWnd = CreateWindowExW(WS_EX_LAYERED, wc.lpszClassName, L"t", WS_POPUP, 0, 0, PUS_W, PUS_H, nullptr, nullptr, wc.hInstance, nullptr);
    g_memDC = CreateCompatibleDC(nullptr);
    BITMAPINFO bmi = {};
    bmi.bmiHeader = { sizeof(BITMAPINFOHEADER), PUS_W, -PUS_H, 1, 32, BI_RGB };
    SelectObject(g_memDC, CreateDIBSection(g_memDC, &bmi, DIB_RGB_COLORS, &g_bits, nullptr, 0));

    const char* names[] = { "HP Scroll 60%", "Scroll of Armor 400", "Scroll of 2000 HP", "Scroll of Armor 350",
        "Scroll of Attack", "Speed-Up Potion", "Red Potion", "Blue Potion", "Orange Potion", "NP increase item",
        "Master's teaching", "Premium Potion HP", "Premium Potion MP" };
    UINT32 icons[] = { 39929500, 81064000, 37925800, 70000200, 70000900, 0, 12345, 39929500, 81064000, 37925800, 70000200, 70000900, 1 };
    for (int i = 0; i < 13; i++)
    {
        SPusItem it; it.ID = i + 1; it.Name = names[i]; it.Price = 99 + i * 150; it.Cat = (BYTE)(1 + i % 5);
        it.BuyCount = 1; it.PriceType = i == 3 ? 1 : 0; it.IconID = icons[i];
        g_items.push_back(it);
    }
    const char* cats[] = { "Scrolls", "Premium-Other", "Trina-Pot-Scroll", "Cospre items", "TL & Knight KC" };
    for (int i = 0; i < 5; i++) { SPusCategory c; c.ID = i + 1; c.Name = cats[i]; g_cats.push_back(c); }
    g_kc = 75744; g_tl = 207812; g_listRequested = true;
    RebuildFiltered();
    g_hover = C_BUY0 + 1;
    if (argc > 2) { g_confirmIdx = 2; g_qty = 3; }
    Paint();

    FILE* f = nullptr;
    fopen_s(&f, argv[1], "wb");
    fwrite(g_bits, 4, (size_t)PUS_W * PUS_H, f);
    fclose(f);
    printf("wrote %s\n", argv[1]);
    return 0;
}
