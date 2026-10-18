#include "PrintColored.h"

void PrintColored(const tstring& message, WORD color) {
    HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);
    CONSOLE_SCREEN_BUFFER_INFO consoleInfo;
    WORD saved_attributes;

    // Mevcut konsol renklerini kaydet
    GetConsoleScreenBufferInfo(hConsole, &consoleInfo);
    saved_attributes = consoleInfo.wAttributes;

    // Yeni rengi ayarla ve mesajý yazdýr
    SetConsoleTextAttribute(hConsole, color);
    _tprintf(_T("%s"), message.c_str());

    // Orijinal renkleri geri yükle
    SetConsoleTextAttribute(hConsole, saved_attributes);
}
