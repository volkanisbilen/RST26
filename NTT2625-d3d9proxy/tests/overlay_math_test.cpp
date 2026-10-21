// cl /nologo /EHsc /I.. overlay_math_test.cpp && overlay_math_test.exe
#include "../overlay_math.h"
#include <stdio.h>
static int fails = 0;
#define CHECK(c) do { if (!(c)) { printf("FAIL line %d: %s\n", __LINE__, #c); fails++; } } while (0)
int main()
{
    CHECK(ovm::Pow2Ceil(1) == 1); CHECK(ovm::Pow2Ceil(3) == 4); CHECK(ovm::Pow2Ceil(256) == 256); CHECK(ovm::Pow2Ceil(257) == 512);
    // 3x2 bitmap, top-down: visible rows r0,r1
    unsigned td[6] = { 0xFF000001, 0, 0x01000002, 0x80000003, 0, 0 };
    unsigned dst[6], sh[6];
    ovm::CopyRows((const unsigned char*)td, 12, false, 2, 0, 0, 3, 2, dst, sh);
    CHECK(memcmp(dst, td, 24) == 0);
    CHECK(sh[0] == 0x01000000u && sh[1] == 0 && sh[2] == 0x01000000u && sh[3] == 0x01000000u && sh[4] == 0 && sh[5] == 0);
    // same image bottom-up in memory: row order reversed
    unsigned bu[6] = { 0x80000003, 0, 0, 0xFF000001, 0, 0x01000002 };
    ovm::CopyRows((const unsigned char*)bu, 12, true, 2, 0, 0, 3, 2, dst, nullptr);
    CHECK(memcmp(dst, td, 24) == 0);
    // sub-rect: x=1,y=1,w=2,h=1 of top-down
    unsigned d2[2];
    ovm::CopyRows((const unsigned char*)td, 12, false, 2, 1, 1, 2, 1, d2, nullptr);
    CHECK(d2[0] == 0 && d2[1] == 0);
    ovm::CopyRows((const unsigned char*)td, 12, false, 2, 0, 1, 2, 1, d2, nullptr);
    CHECK(d2[0] == 0x80000003u);
    unsigned m[6]; ovm::MaskFrom(td, m, 6); CHECK(m[0] == 0x01000000u && m[1] == 0);
    // mapping: client at (100,50), backbuffer 2x client
    ovm::Quad q = ovm::MapRect(110, 60, 20, 10, 100, 50, 2.0f, 2.0f);
    CHECK(q.x0 == 19.5f && q.y0 == 19.5f && q.x1 == 59.5f && q.y1 == 39.5f);
    CHECK(ovm::QuadVisible(q, 1024, 768));
    ovm::Quad off = ovm::MapRect(-500, 0, 100, 100, 0, 0, 1, 1);
    CHECK(!ovm::QuadVisible(off, 1024, 768));
    printf(fails ? "FAILED\n" : "all ok\n");
    return fails;
}
