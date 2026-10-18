// Item icons straight from the client's UI\ui.src: itemicon_<A>_<BBBB>_<CC>_<D>.dxt for icon id ABBBBCCD.
// Entries may be JstKO-encrypted (uif_crypt.py cipher) and the NTF texture itself is RC4-scrambled (ver 7),
// exactly as build_cr_assets.py reads it. Result: 45x45 premultiplied BGRA, the same as a JstKO\pus_ui\icons\<id>.pus.
// Used by the Collection Race panel so reward icons need no pre-generated files.
#include <windows.h>
#include <stdio.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <map>
#include <mutex>
#include <algorithm>

namespace
{
    std::mutex g_mu;
    bool g_loaded = false;
    std::map<std::string, std::pair<uint32_t, uint32_t>> g_pack;   // lower-case name -> (offset, size)
    std::string g_src;

    const uint32_t K_SEED = 0x48A7C3D1, K_WORD = 0x9E3779B9, K_TAG = 0x5BD1E995;
    const BYTE RC4_KEY[16] = { 0xF5, 0x06, 0x21, 0x61, 0x7D, 0x29, 0x19, 0x71, 0x5C, 0x81, 0x8D, 0x9D, 0xE9, 0xC9, 0xFD, 0x20 };

    uint32_t Mix(uint32_t x)
    {
        x ^= x >> 16; x *= 0x7FEB352D; x ^= x >> 15; x *= 0x846CA68B; x ^= x >> 16; return x;
    }

    // JstKO pack entry: [payload ^ keystream][u32 nonce][u32 tag]; plain entries pass through
    void HgDecrypt(std::vector<BYTE>& e)
    {
        size_t size = e.size();
        if (size < 8) return;
        uint32_t nonce, tag;
        memcpy(&nonce, &e[size - 8], 4); memcpy(&tag, &e[size - 4], 4);
        if (tag != Mix(Mix(nonce + K_TAG) ^ (uint32_t)size)) return;
        uint32_t sd = Mix(Mix(nonce ^ K_SEED) + (uint32_t)size);
        size_t n = size - 8;
        for (size_t p = 0; p < n; p++)
        {
            uint32_t w = Mix(sd ^ (uint32_t)(((p >> 2) * K_WORD + 1) & 0xFFFFFFFF));
            e[p] ^= (BYTE)((w >> ((p & 3) * 8)) & 0xFF);
        }
        e.resize(n);
    }

    void Rc4(BYTE* data, size_t n)
    {
        BYTE S[256]; int j = 0;
        for (int i = 0; i < 256; i++) S[i] = (BYTE)i;
        for (int i = 0; i < 256; i++) { j = (j + S[i] + RC4_KEY[i % 16]) & 255; std::swap(S[i], S[j]); }
        int a = 0, b = 0;
        for (size_t k = 0; k < n; k++)
        {
            a = (a + 1) & 255; b = (b + S[a]) & 255; std::swap(S[a], S[b]);
            data[k] ^= S[(S[a] + S[b]) & 255];
        }
    }

    bool LoadIndex()
    {
        if (g_loaded) return !g_pack.empty();
        g_loaded = true;
        char exe[MAX_PATH];
        GetModuleFileNameA(nullptr, exe, MAX_PATH);
        char* slash = strrchr(exe, '\\');
        if (!slash) return false;
        *slash = 0;
        std::string dir = exe;
        g_src = dir + "\\UI\\ui.src";
        FILE* f = nullptr;
        if (fopen_s(&f, (dir + "\\UI\\ui.hdr").c_str(), "rb") != 0 || !f) return false;
        std::vector<BYTE> h;
        BYTE buf[65536]; size_t got;
        while ((got = fread(buf, 1, sizeof(buf), f)) > 0) h.insert(h.end(), buf, buf + got);
        fclose(f);
        size_t o = 8;
        while (o + 2 <= h.size())
        {
            uint16_t l; memcpy(&l, &h[o], 2);
            if (o + 2 + l + 8 > h.size()) break;
            std::string nm((const char*)&h[o + 2], l);
            for (char& c : nm) c = (char)tolower((unsigned char)c);
            uint32_t off, sz; memcpy(&off, &h[o + 2 + l], 4); memcpy(&sz, &h[o + 2 + l + 4], 4);
            g_pack[nm] = { off, sz };
            o += 2 + l + 8;
        }
        return !g_pack.empty();
    }

    bool ReadEntry(const std::string& name, std::vector<BYTE>& out)
    {
        auto it = g_pack.find(name);
        if (it == g_pack.end()) return false;
        FILE* f = nullptr;
        if (fopen_s(&f, g_src.c_str(), "rb") != 0 || !f) return false;
        out.resize(it->second.second);
        bool ok = fseek(f, (long)it->second.first, SEEK_SET) == 0 && fread(out.data(), 1, out.size(), f) == out.size();
        fclose(f);
        if (ok) HgDecrypt(out);
        return ok;
    }

    std::string IconFile(uint32_t id)
    {
        std::string s = std::to_string(id);
        if (s.size() < 8) return std::string();
        char b[96];
        sprintf_s(b, "itemicon_%s_%s_%s_%s.dxt", s.substr(0, s.size() - 7).c_str(), s.substr(s.size() - 7, 4).c_str(),
                  s.substr(s.size() - 3, 2).c_str(), s.substr(s.size() - 1).c_str());
        return b;
    }

    uint32_t C565(uint16_t c, BYTE* rgb)
    {
        rgb[0] = (BYTE)(((c >> 11) & 31) * 255 / 31); rgb[1] = (BYTE)(((c >> 5) & 63) * 255 / 63); rgb[2] = (BYTE)((c & 31) * 255 / 31);
        return 0;
    }

    // decodes the whole texture to RGBA8 (r,g,b,a bytes)
    bool Decode(const BYTE* d, size_t len, uint32_t w, uint32_t h, uint32_t fmt, std::vector<BYTE>& rgba)
    {
        rgba.assign((size_t)w * h * 4, 0);
        if (fmt == 21 || fmt == 22)
        {
            if (len < (size_t)w * h * 4) return false;
            for (size_t i = 0; i < (size_t)w * h; i++)
            {
                rgba[i * 4] = d[i * 4 + 2]; rgba[i * 4 + 1] = d[i * 4 + 1]; rgba[i * 4 + 2] = d[i * 4];
                rgba[i * 4 + 3] = fmt == 21 ? d[i * 4 + 3] : 255;
            }
            return true;
        }
        if (fmt == 25 || fmt == 26)
        {
            if (len < (size_t)w * h * 2) return false;
            for (size_t i = 0; i < (size_t)w * h; i++)
            {
                uint16_t v; memcpy(&v, d + i * 2, 2);
                if (fmt == 25) { rgba[i * 4] = (BYTE)(((v >> 10) & 31) * 255 / 31); rgba[i * 4 + 1] = (BYTE)(((v >> 5) & 31) * 255 / 31); rgba[i * 4 + 2] = (BYTE)((v & 31) * 255 / 31); rgba[i * 4 + 3] = (v & 0x8000) ? 255 : 0; }
                else { rgba[i * 4 + 3] = (BYTE)(((v >> 12) & 15) * 17); rgba[i * 4] = (BYTE)(((v >> 8) & 15) * 17); rgba[i * 4 + 1] = (BYTE)(((v >> 4) & 15) * 17); rgba[i * 4 + 2] = (BYTE)((v & 15) * 17); }
            }
            return true;
        }
        int kind = fmt == 0x31545844 ? 1 : fmt == 0x33545844 ? 2 : fmt == 0x35545844 ? 3 : 0;
        if (!kind) return false;
        size_t bs = kind == 1 ? 8 : 16, bw = w / 4, bh = h / 4;
        if (len < bw * bh * bs) return false;
        for (size_t by = 0; by < bh; by++)
            for (size_t bx = 0; bx < bw; bx++)
            {
                const BYTE* b = d + (by * bw + bx) * bs;
                BYTE alpha[16]; for (int i = 0; i < 16; i++) alpha[i] = 255;
                if (kind == 2)
                    for (int i = 0; i < 16; i++) alpha[i] = (BYTE)(((b[i / 2] >> ((i & 1) * 4)) & 15) * 17);
                else if (kind == 3)
                {
                    BYTE a0 = b[0], a1 = b[1], tab[8]; tab[0] = a0; tab[1] = a1;
                    if (a0 > a1) for (int i = 1; i < 7; i++) tab[i + 1] = (BYTE)(((7 - i) * a0 + i * a1) / 7);
                    else { for (int i = 1; i < 5; i++) tab[i + 1] = (BYTE)(((5 - i) * a0 + i * a1) / 5); tab[6] = 0; tab[7] = 255; }
                    uint64_t bits = 0; for (int i = 0; i < 6; i++) bits |= (uint64_t)b[2 + i] << (8 * i);
                    for (int i = 0; i < 16; i++) alpha[i] = tab[(bits >> (3 * i)) & 7];
                }
                const BYTE* c = b + (kind == 1 ? 0 : 8);
                uint16_t c0, c1; memcpy(&c0, c, 2); memcpy(&c1, c + 2, 2);
                BYTE col[4][4]; C565(c0, col[0]); C565(c1, col[1]); col[0][3] = col[1][3] = col[2][3] = col[3][3] = 255;
                if (c0 > c1 || kind != 1)
                    for (int k = 0; k < 3; k++) { col[2][k] = (BYTE)((2 * col[0][k] + col[1][k]) / 3); col[3][k] = (BYTE)((col[0][k] + 2 * col[1][k]) / 3); }
                else
                {
                    for (int k = 0; k < 3; k++) { col[2][k] = (BYTE)((col[0][k] + col[1][k]) / 2); col[3][k] = 0; }
                    col[3][3] = 0;
                }
                uint32_t idx; memcpy(&idx, c + 4, 4);
                for (int py = 0; py < 4; py++)
                    for (int px = 0; px < 4; px++)
                    {
                        int i = py * 4 + px, ci = (idx >> (2 * i)) & 3;
                        BYTE* o = &rgba[(((by * 4 + py) * w) + bx * 4 + px) * 4];
                        o[0] = col[ci][0]; o[1] = col[ci][1]; o[2] = col[ci][2];
                        o[3] = kind == 1 ? col[ci][3] : alpha[i];
                    }
            }
        return true;
    }

    bool LoadTexture(const std::string& name, std::vector<BYTE>& rgba, uint32_t& w, uint32_t& h)
    {
        std::vector<BYTE> e;
        if (!ReadEntry(name, e) || e.size() < 32) return false;
        int32_t nl; memcpy(&nl, &e[0], 4);
        if (nl < 0 || (size_t)nl + 4 + 20 > e.size()) return false;
        size_t o = 4 + (size_t)nl;
        if (memcmp(&e[o], "NTF", 3) != 0) return false;
        BYTE ver = e[o + 3];
        uint32_t mip, fmt; memcpy(&w, &e[o + 4], 4); memcpy(&h, &e[o + 8], 4); memcpy(&fmt, &e[o + 12], 4); memcpy(&mip, &e[o + 16], 4);
        o += 20;
        if (!w || !h || w > 2048 || h > 2048) return false;
        size_t pitch, rows;
        if (fmt == 21 || fmt == 22) { pitch = (size_t)w * 4; rows = h; }
        else if (fmt == 25 || fmt == 26) { pitch = (size_t)w * 2; rows = h; }
        else if (fmt == 0x31545844 || fmt == 0x33545844 || fmt == 0x35545844) { pitch = (size_t)(w / 4) * (fmt == 0x31545844 ? 8 : 16); rows = h / 4; }
        else return false;
        if (o + pitch * rows > e.size()) return false;
        std::vector<BYTE> data(e.begin() + o, e.begin() + o + pitch * rows);
        bool dxt = fmt != 21 && fmt != 22 && fmt != 25 && fmt != 26;
        if (ver == 7)
        {
            if (dxt) Rc4(data.data(), data.size());
            else for (size_t y = 0; y < rows; y++) Rc4(&data[y * pitch], pitch);
        }
        return Decode(data.data(), data.size(), w, h, fmt, rgba);
    }
}

// out45: 45*45*4 bytes, premultiplied BGRA. false = no such icon in the pack.
bool PackIcon_Load45(unsigned iconId, unsigned char* out45)
{
    std::lock_guard<std::mutex> lk(g_mu);
    if (!iconId || !LoadIndex()) return false;
    std::vector<BYTE> rgba; uint32_t w = 0, h = 0;
    std::string n = IconFile(iconId);
    bool ok = !n.empty() && LoadTexture(n, rgba, w, h);
    if (!ok)
    {
        n = IconFile(iconId / 10 * 10);   // colour variant -> base icon
        ok = !n.empty() && LoadTexture(n, rgba, w, h);
    }
    if (!ok) return false;
    for (int y = 0; y < 45; y++)
        for (int x = 0; x < 45; x++)
        {
            BYTE r = 0, g = 0, b = 0, a = 0;
            if ((uint32_t)x < w && (uint32_t)y < h) { const BYTE* p = &rgba[((size_t)y * w + x) * 4]; r = p[0]; g = p[1]; b = p[2]; a = p[3]; }
            BYTE* o = out45 + (y * 45 + x) * 4;
            o[0] = (BYTE)(b * a / 255); o[1] = (BYTE)(g * a / 255); o[2] = (BYTE)(r * a / 255); o[3] = a;
        }
    return true;
}
