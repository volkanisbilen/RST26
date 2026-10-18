// Obfuscates d3d9proxy.log so a player who opens the file in Notepad doesn't get a readable
// play-by-play of hook addresses, item/account names and internal state -- not meant to resist a
// determined attacker (it's a repeating-key XOR + base64, same spirit as this repo's xorstr.h),
// just to keep the log from being plain text. Decrypt with log_decrypt.py (same KEY).
#include <windows.h>
#include <string>

namespace
{
    // Must match KEY in log_decrypt.py byte for byte. Rotate both together if it ever leaks.
    const unsigned char KEY[] = "JstKO-2625-log-obfuscation-v1-9f1c7a3e-do-not-leak";
    const size_t KEY_LEN = sizeof(KEY) - 1;

    const char* B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    std::string Base64Encode(const unsigned char* data, size_t len)
    {
        std::string out;
        out.reserve((len + 2) / 3 * 4);
        size_t i = 0;
        for (; i + 3 <= len; i += 3)
        {
            unsigned v = (data[i] << 16) | (data[i + 1] << 8) | data[i + 2];
            out += B64[(v >> 18) & 0x3F];
            out += B64[(v >> 12) & 0x3F];
            out += B64[(v >> 6) & 0x3F];
            out += B64[v & 0x3F];
        }
        size_t rem = len - i;
        if (rem == 1)
        {
            unsigned v = data[i] << 16;
            out += B64[(v >> 18) & 0x3F];
            out += B64[(v >> 12) & 0x3F];
            out += "==";
        }
        else if (rem == 2)
        {
            unsigned v = (data[i] << 16) | (data[i + 1] << 8);
            out += B64[(v >> 18) & 0x3F];
            out += B64[(v >> 12) & 0x3F];
            out += B64[(v >> 6) & 0x3F];
            out += "=";
        }
        return out;
    }
}

// One log line in -> one base64 blob out (no embedded '\n' so the log stays line-per-entry).
std::string LogObfuscate(const char* text, size_t len)
{
    std::string buf(len, '\0');
    for (size_t i = 0; i < len; i++)
        buf[i] = (char)((unsigned char)text[i] ^ KEY[i % KEY_LEN]);
    return Base64Encode((const unsigned char*)buf.data(), len);
}
