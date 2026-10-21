// overlay_math.h - pure helpers of the D3D overlay compositor (overlay_d3d.cpp). No Windows / D3D dependency,
// so tests\overlay_math_test.cpp can build and run them stand-alone.
#pragma once
#include <string.h>

namespace ovm
{
    inline unsigned Pow2Ceil(unsigned v) { unsigned p = 1; while (p < v && p < (1u << 30)) p <<= 1; return p; }

    // Copies a w x h region (top-left sx,sy in visible/top-down coordinates) of a 32-bit premultiplied ARGB bitmap
    // into dst (tightly packed, top row first). bottomUp = DIB stored bottom row first (positive biHeight), memH = rows
    // in memory. If shadow is given it receives the hit-test mask: 0x01000000 where the source alpha is non-zero, else 0.
    // A layered window is click-through only where its alpha is 0, so the near-invisible mask keeps the exact hit shape.
    inline void CopyRows(const unsigned char* base, int pitch, bool bottomUp, int memH, int sx, int sy, int w, int h,
                         unsigned* dst, unsigned* shadow)
    {
        for (int y = 0; y < h; y++)
        {
            int memRow = bottomUp ? (memH - 1 - (sy + y)) : (sy + y);
            const unsigned* row = (const unsigned*)(base + (size_t)memRow * (size_t)pitch) + sx;
            memcpy(dst + (size_t)y * w, row, (size_t)w * 4);
            if (shadow)
            {
                unsigned* s = shadow + (size_t)y * w;
                for (int x = 0; x < w; x++) s[x] = (row[x] >> 24) ? 0x01000000u : 0u;
            }
        }
    }

    inline void MaskFrom(const unsigned* pix, unsigned* shadow, int n)
    {
        for (int i = 0; i < n; i++) shadow[i] = (pix[i] >> 24) ? 0x01000000u : 0u;
    }

    struct Quad { float x0, y0, x1, y1; };

    // Screen-space window rect -> backbuffer pixel quad (client origin ox,oy in screen coords; sx,sy = backbuffer / client scale).
    // The -0.5 is the D3D9 texel/pixel centre offset for pre-transformed vertices.
    inline Quad MapRect(int left, int top, int w, int h, int ox, int oy, float sx, float sy)
    {
        Quad q;
        q.x0 = (float)(left - ox) * sx - 0.5f;
        q.y0 = (float)(top - oy) * sy - 0.5f;
        q.x1 = (float)(left + w - ox) * sx - 0.5f;
        q.y1 = (float)(top + h - oy) * sy - 0.5f;
        return q;
    }

    inline bool QuadVisible(const Quad& q, float bbW, float bbH)
    {
        return q.x1 > 0 && q.y1 > 0 && q.x0 < bbW && q.y0 < bbH;
    }
}
