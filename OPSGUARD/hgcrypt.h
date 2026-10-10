// Stream cipher for HopeGuard-encrypted pack entries (UI\ui.src *.uif).
// Keep in sync with PatchTools\uif_crypt.py.
//
// Encrypted entry = [payload ^ keystream][u32 nonce][u32 tag]; the .hdr size covers all of it.
// The keystream is addressed by byte offset inside the entry, so the game can read in any chunks.
#pragma once
#include <stdint.h>

namespace hg {

const uint32_t kSeed = 0x48A7C3D1, kWord = 0x9E3779B9, kTag = 0x5BD1E995;
const uint32_t kTrailer = 8;

inline uint32_t Mix(uint32_t x)
{
    x ^= x >> 16; x *= 0x7FEB352D;
    x ^= x >> 15; x *= 0x846CA68B;
    x ^= x >> 16;
    return x;
}

// size = entry size as stored in the .hdr (payload + trailer)
inline uint32_t Seed(uint32_t nonce, uint32_t size) { return Mix(Mix(nonce ^ kSeed) + size); }
inline uint32_t Tag(uint32_t nonce, uint32_t size)  { return Mix(Mix(nonce + kTag) ^ size); }

// Loose proxy assets (HopeGuard\*_ui\*.pus) are XORed in place, same size, no trailer: the key comes
// from the file size and the lower-case file name, and a decrypted "PUSI" magic tells them from plain ones.
// Keep in sync with PatchTools\asset_crypt.py.
const uint32_t kAsset = 0xC2B2AE35;

inline uint32_t NameHash(const char* lowerName)
{
    uint32_t h = 0x811C9DC5;
    for (; *lowerName; lowerName++) { h ^= (uint8_t)*lowerName; h *= 0x01000193; }
    return h;
}
inline uint32_t AssetSeed(uint32_t size, uint32_t nameHash) { return Mix(Mix(size ^ kAsset) + nameHash); }

// XORs n bytes that sit at offset pos inside the entry.
inline void Crypt(uint8_t* p, uint32_t n, uint32_t pos, uint32_t seed)
{
    uint32_t i = 0;
    for (; i < n && ((pos + i) & 3); i++)                       // up to the next keystream word
        p[i] ^= (uint8_t)(Mix(seed ^ (((pos + i) >> 2) * kWord + 1)) >> (((pos + i) & 3) * 8));
    for (; i + 4 <= n; i += 4) {                                // whole words (textures are megabytes)
        uint32_t w = Mix(seed ^ (((pos + i) >> 2) * kWord + 1));
        p[i] ^= (uint8_t)w; p[i + 1] ^= (uint8_t)(w >> 8); p[i + 2] ^= (uint8_t)(w >> 16); p[i + 3] ^= (uint8_t)(w >> 24);
    }
    for (; i < n; i++)
        p[i] ^= (uint8_t)(Mix(seed ^ (((pos + i) >> 2) * kWord + 1)) >> (((pos + i) & 3) * 8));
}

}
