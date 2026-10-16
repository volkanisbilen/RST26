// Client integrity handshake with the server (hook sub-op 0xF4, see
// ko-game/src/handler/client_integrity.rs). Only this DLL knows KEY (it must match the
// server's copy exactly), so a repacked client or a home-made bot can't answer the
// challenge; module_guard.cpp's injection flag rides along in the same reply.
//   S2C [1][u8 nonce_len][nonce bytes]                          - challenge, once at game entry
//   C2S [2][32 bytes HMAC-SHA256(KEY, nonce)][u8 tamper]        - reply
//   C2S [3]                                                     - async: a module was injected after startup
#include <windows.h>
#include <bcrypt.h>
#include <string.h>
#include <vector>
#pragma comment(lib, "bcrypt.lib")

void Log(const char* msg);
void Drop_SendRaw(const std::vector<BYTE>& p);
bool ModuleGuard_Tampered();

namespace
{
    const BYTE SUB = 0xF4;
    // Must match KEY in crates/ko-game/src/handler/client_integrity.rs byte for byte.
    const char* KEY = "JstKO-2625-client-integrity-v1-7f3c9a1e";

    bool HmacSha256(const BYTE* data, DWORD len, BYTE out32[32])
    {
        bool ok = false;
        BCRYPT_ALG_HANDLE alg = nullptr;
        if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, nullptr, BCRYPT_ALG_HANDLE_HMAC_FLAG) == 0)
        {
            BCRYPT_HASH_HANDLE hash = nullptr;
            if (BCryptCreateHash(alg, &hash, nullptr, 0, (PUCHAR)KEY, (ULONG)strlen(KEY), 0) == 0)
            {
                if (BCryptHashData(hash, (PUCHAR)data, len, 0) == 0 &&
                    BCryptFinishHash(hash, out32, 32, 0) == 0)
                {
                    ok = true;
                }
                BCryptDestroyHash(hash);
            }
            BCryptCloseAlgorithmProvider(alg, 0);
        }
        return ok;
    }
}

// Sent the moment module_guard.cpp notices an injected module, not just at login.
void Integrity_SendAsyncAlert()
{
    Drop_SendRaw({ 0xE9, SUB, 3 });
}

void Integrity_OnRecv(const BYTE* p, size_t len)
{
    if (len < 4 || p[0] != 0xE9 || p[1] != SUB || p[2] != 1) return;
    BYTE nlen = p[3];
    if (len < (size_t)4 + nlen) return;
    BYTE mac[32] = {};
    if (!HmacSha256(p + 4, nlen, mac))
    {
        Log("INTEGRITY: HMAC computation failed, not answering the server challenge");
        return;
    }
    std::vector<BYTE> out = { 0xE9, SUB, 2 };
    out.insert(out.end(), mac, mac + 32);
    out.push_back(ModuleGuard_Tampered() ? 1 : 0);
    Drop_SendRaw(out);
}
