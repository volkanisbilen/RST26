// overlay_d3d.cpp - central overlay compositor: our layered-window panels are drawn INSIDE the D3D frame.
//
// Why: every panel (daily, CR, drop, GM, tooltips ...) is a separate top-level WS_EX_LAYERED window that GDI-draws into a 32-bit
// premultiplied DIB and shows it with UpdateLayeredWindow. Discord / OBS game capture and the game's own screenshot only see the
// D3D backbuffer, so none of the panels showed up there.
//
// How (no panel was edited):
//  1. Ov_Init() patches OUR OWN import slot of user32!UpdateLayeredWindow, so every panel's call lands in Ov_ULW. It keeps a copy
//     of the pixels (+ constant alpha) per HWND. Position / visibility / z-order are read from the real window at draw time.
//  2. The IDirect3D9::CreateDevice vtable slot is hooked (Direct3DCreate9 in d3d9proxy.cpp calls Ov_OnD3D9Created), and from the
//     device it creates, IDirect3DDevice9::Present. Right before the real Present the overlays are drawn (BeginScene, textured
//     quads, EndScene) on the backbuffer, state saved / restored with a state block. Capture hooks sit below us, so they see them.
//  3. The real windows stay (input, WndProcs), but once the compositor is live they are shown with a hit-test mask instead of the
//     picture: alpha 1/255 (black) where the panel is not transparent, 0 elsewhere. A layered window is click-through only where
//     alpha is 0, so clicks keep their exact shape and nothing is drawn twice.
//  4. Fallback: JstKO\overlay.ini [overlay] d3d=0 disables everything (pure pass-through). If the device side fails (exceptions,
//     texture / draw errors) the compositor switches itself off and every real window gets its real picture back.
//
// Textures are D3DPOOL_MANAGED (survive Reset, nothing to release in the default pool). No D3D9 reference is kept on the device.
#include <windows.h>
#include <d3d9.h>
#include <stdio.h>
#include <string.h>
#include "overlay_math.h"

void Log(const char* msg);
HWND Proxy_GameWnd();

extern "C" IMAGE_DOS_HEADER __ImageBase;

namespace
{
    const int MAX_OV = 64;
    const int MAX_DIM = 4096;

    typedef BOOL(WINAPI* PFN_ULW)(HWND, HDC, POINT*, SIZE*, HDC, POINT*, COLORREF, BLENDFUNCTION*, DWORD);
    PFN_ULW g_realULW = nullptr;

    struct Ov
    {
        HWND h = nullptr;
        int w = 0, hgt = 0;
        unsigned* pix = nullptr;        // w*hgt premultiplied ARGB copy (guarded by g_lock)
        size_t pixCap = 0;              // capacity in pixels
        BYTE ca = 255;                  // SourceConstantAlpha of the last update
        bool hasPix = false, dirty = false, bypass = false;
        // shadow (hit mask or raw picture) handed to the real UpdateLayeredWindow; owned by whoever holds inUse
        HDC shDC = nullptr; HBITMAP shBmp = nullptr; HGDIOBJ shOld = nullptr; unsigned* shBits = nullptr; int shW = 0, shH = 0;
        volatile LONG inUse = 0;
        // render thread only
        IDirect3DTexture9* tex = nullptr; int texW = 0, texH = 0;
    };

    SRWLOCK g_lock = SRWLOCK_INIT;
    Ov* g_ovs[MAX_OV];
    volatile LONG g_ovCount = 0;

    volatile LONG g_cfg = -1;           // -1 unread, 0 off, 1 on
    volatile LONG g_inited = 0;         // Ov_Init ran (game host only)
    volatile LONG g_live = 0;           // compositor draws -> real windows show only the hit mask
    volatile LONG g_failed = 0;         // device side gave up -> permanent pass-through
    volatile LONG g_maxTex = 0;
    volatile LONG g_reapplyRunning = 0;

    // device side (render thread only)
    IDirect3DDevice9* g_dev = nullptr;
    bool g_ready = false;
    HWND g_focus = nullptr;
    int g_drawFails = 0, g_exceptions = 0;
    bool g_loggedFirst = false;

    typedef HRESULT(STDMETHODCALLTYPE* PFN_CreateDevice)(IDirect3D9*, UINT, D3DDEVTYPE, HWND, DWORD, D3DPRESENT_PARAMETERS*, IDirect3DDevice9**);
    typedef HRESULT(STDMETHODCALLTYPE* PFN_Present)(IDirect3DDevice9*, const RECT*, const RECT*, HWND, const RGNDATA*);
    PFN_CreateDevice g_origCreate = nullptr;
    PFN_Present g_origPresent = nullptr;
    void** g_hookedDevVt = nullptr;

    bool Enabled()
    {
        if (g_cfg < 0)
        {
            char p[MAX_PATH]; GetModuleFileNameA(nullptr, p, MAX_PATH);
            char* s = strrchr(p, '\\');
            LONG v = 1;
            if (s) { *(s + 1) = 0; strcat_s(p, "JstKO\\overlay.ini"); v = GetPrivateProfileIntA("overlay", "d3d", 1, p) ? 1 : 0; }
            InterlockedCompareExchange(&g_cfg, v, -1);
            char b[96]; sprintf_s(b, "OVERLAY: [overlay] d3d=%ld", g_cfg); Log(b);
        }
        return g_cfg == 1;
    }

    // ---- shadow bitmap -----------------------------------------------------------------------------
    bool EnsureShadow(Ov* o, int w, int h)
    {
        if (o->shDC && o->shW == w && o->shH == h) return true;
        if (o->shDC)
        {
            SelectObject(o->shDC, o->shOld); DeleteObject(o->shBmp); DeleteDC(o->shDC);
            o->shDC = nullptr; o->shBmp = nullptr; o->shBits = nullptr; o->shW = o->shH = 0;
        }
        BITMAPINFO bi = {}; bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER); bi.bmiHeader.biWidth = w; bi.bmiHeader.biHeight = -h;
        bi.bmiHeader.biPlanes = 1; bi.bmiHeader.biBitCount = 32; bi.bmiHeader.biCompression = BI_RGB;
        void* bits = nullptr;
        HBITMAP bmp = CreateDIBSection(nullptr, &bi, DIB_RGB_COLORS, &bits, nullptr, 0);
        if (!bmp || !bits) { if (bmp) DeleteObject(bmp); return false; }
        HDC dc = CreateCompatibleDC(nullptr);
        if (!dc) { DeleteObject(bmp); return false; }
        o->shOld = SelectObject(dc, bmp);
        o->shDC = dc; o->shBmp = bmp; o->shBits = (unsigned*)bits; o->shW = w; o->shH = h;
        return true;
    }

    bool EnsurePix(Ov* o, int w, int h)   // g_lock held
    {
        size_t need = (size_t)w * h;
        if (o->pixCap < need)
        {
            unsigned* np = (unsigned*)realloc(o->pix, need * 4);
            if (!np) return false;
            o->pix = np; o->pixCap = need;
        }
        if (o->w != w || o->hgt != h) { o->w = w; o->hgt = h; }
        return true;
    }

    Ov* FindOrCreate(HWND h)
    {
        AcquireSRWLockExclusive(&g_lock);
        Ov* r = nullptr;
        for (int i = 0; i < g_ovCount; i++) if (g_ovs[i]->h == h) { r = g_ovs[i]; break; }
        if (!r && g_ovCount < MAX_OV)
        {
            r = new Ov(); r->h = h;
            g_ovs[g_ovCount] = r;
            InterlockedIncrement(&g_ovCount);
        }
        ReleaseSRWLockExclusive(&g_lock);
        return r;
    }

    // Reads the source bitmap of hdcSrc into o->pix (and the mask into the shadow). false = could not (caller passes the call through).
    bool CaptureSource(Ov* o, HDC src, const POINT* sp, int w, int h, bool maskShadow)
    {
        int sx = sp ? sp->x : 0, sy = sp ? sp->y : 0;
        if (sx < 0 || sy < 0) return false;
        HBITMAP hb = (HBITMAP)GetCurrentObject(src, OBJ_BITMAP);
        DIBSECTION ds;
        const unsigned char* base = nullptr; int pitch = 0, memH = 0; bool bottomUp = false;
        if (hb && GetObjectW(hb, sizeof(ds), &ds) == sizeof(ds) && ds.dsBm.bmBits && ds.dsBm.bmBitsPixel == 32)
        {
            base = (const unsigned char*)ds.dsBm.bmBits; pitch = ds.dsBm.bmWidthBytes; memH = ds.dsBm.bmHeight;
            bottomUp = ds.dsBmih.biHeight > 0;
            if (sx + w > ds.dsBm.bmWidth || sy + h > memH) return false;
        }
        else
        {   // not a 32-bit DIB section: let GDI copy it into our shadow DIB first
            if (!o->shDC || !BitBlt(o->shDC, 0, 0, w, h, src, sx, sy, SRCCOPY)) return false;
            GdiFlush();
            base = (const unsigned char*)o->shBits; pitch = w * 4; memH = h; bottomUp = false; sx = 0; sy = 0;
        }
        AcquireSRWLockExclusive(&g_lock);
        bool ok = EnsurePix(o, w, h);
        if (ok)
        {
            if (base == (const unsigned char*)o->shBits)
            {   // fallback case: shadow holds the raw picture; copy to pix, then convert the shadow to the mask if wanted
                ovm::CopyRows(base, pitch, bottomUp, memH, sx, sy, w, h, o->pix, nullptr);
                if (maskShadow) ovm::MaskFrom(o->pix, o->shBits, w * h);
            }
            else
                ovm::CopyRows(base, pitch, bottomUp, memH, sx, sy, w, h, o->pix, maskShadow ? o->shBits : nullptr);
            o->hasPix = true; o->dirty = true;
        }
        ReleaseSRWLockExclusive(&g_lock);
        return ok;
    }

    BOOL CallReal(HWND hWnd, HDC hdcDst, POINT* pptDst, SIZE* psize, HDC hdcSrc, POINT* pptSrc, COLORREF crKey, BLENDFUNCTION* pb, DWORD fl)
    {
        return g_realULW(hWnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pb, fl);
    }

    BOOL WINAPI Ov_ULW(HWND hWnd, HDC hdcDst, POINT* pptDst, SIZE* psize, HDC hdcSrc, POINT* pptSrc, COLORREF crKey, BLENDFUNCTION* pb, DWORD fl)
    {
        if (!g_realULW) return FALSE;
        if (!Enabled() || g_failed || !hdcSrc || !psize || !pb || !(fl & ULW_ALPHA) || (fl & ULW_COLORKEY) ||
            psize->cx <= 0 || psize->cy <= 0 || psize->cx > MAX_DIM || psize->cy > MAX_DIM || pb->AlphaFormat != AC_SRC_ALPHA)
            return CallReal(hWnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pb, fl);
        Ov* o = FindOrCreate(hWnd);
        if (!o) return CallReal(hWnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pb, fl);
        const int w = psize->cx, h = psize->cy;
        const bool own = InterlockedCompareExchange(&o->inUse, 1, 0) == 0;
        BOOL ret;
        bool captured = false;
        __try
        {
            bool live = g_live != 0;
            bool big = g_maxTex && (w > g_maxTex || h > g_maxTex);
            if (own && EnsureShadow(o, w, h))
            {
                captured = CaptureSource(o, hdcSrc, pptSrc, w, h, live && !big && !o->bypass);
                if (captured) { o->ca = pb->SourceConstantAlpha; if (big) o->bypass = true; }
                else if (live) o->bypass = true;   // unreadable source: this window stays a real, visible window
            }
            if (captured && live && !o->bypass)
            {
                POINT z = { 0, 0 };
                BLENDFUNCTION bf = { AC_SRC_OVER, 0, 255, AC_SRC_ALPHA };
                ret = CallReal(hWnd, hdcDst, pptDst, psize, o->shDC, &z, crKey, &bf, fl);   // hit mask only; the picture goes through D3D
            }
            else if (!own && live && !o->bypass)
                ret = TRUE;   // another thread is updating this window right now; the mask it sets is enough
            else
                ret = CallReal(hWnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pb, fl);
        }
        __except (EXCEPTION_EXECUTE_HANDLER)
        {
            ret = CallReal(hWnd, hdcDst, pptDst, psize, hdcSrc, pptSrc, crKey, pb, fl);
        }
        if (own) InterlockedExchange(&o->inUse, 0);
        return ret;
    }

    // ---- re-apply: switch the real windows between hit mask and raw picture when the compositor goes live / fails ----------
    DWORD WINAPI ReapplyThread(LPVOID)
    {
        for (int pass = 0; pass < 2; pass++)   // twice: a panel that was mid-update the first time gets the new mode as well
        {
            Ov* list[MAX_OV]; int n = 0;
            AcquireSRWLockExclusive(&g_lock);
            for (int i = 0; i < g_ovCount; i++) list[n++] = g_ovs[i];
            ReleaseSRWLockExclusive(&g_lock);
            for (int i = 0; i < n; i++)
            {
                Ov* o = list[i];
                __try
                {
                    if (!o->hasPix || !IsWindow(o->h) || InterlockedCompareExchange(&o->inUse, 1, 0) != 0) continue;
                    bool mask = g_live && !g_failed && !o->bypass;
                    if (EnsureShadow(o, o->w, o->hgt))
                    {
                        AcquireSRWLockExclusive(&g_lock);
                        if (o->shW == o->w && o->shH == o->hgt)
                        {
                            if (mask) ovm::MaskFrom(o->pix, o->shBits, o->w * o->hgt);
                            else memcpy(o->shBits, o->pix, (size_t)o->w * o->hgt * 4);
                        }
                        BYTE ca = o->ca;
                        ReleaseSRWLockExclusive(&g_lock);
                        POINT z = { 0, 0 }; SIZE sz = { o->w, o->hgt };
                        BLENDFUNCTION bf = { AC_SRC_OVER, 0, mask ? (BYTE)255 : ca, AC_SRC_ALPHA };
                        g_realULW(o->h, nullptr, nullptr, &sz, o->shDC, &z, 0, &bf, ULW_ALPHA);
                    }
                    InterlockedExchange(&o->inUse, 0);
                }
                __except (EXCEPTION_EXECUTE_HANDLER) { InterlockedExchange(&o->inUse, 0); }
            }
            Sleep(250);
        }
        InterlockedExchange(&g_reapplyRunning, 0);
        return 0;
    }

    void StartReapply()
    {
        if (InterlockedCompareExchange(&g_reapplyRunning, 1, 0) != 0) return;
        HANDLE t = CreateThread(nullptr, 0, ReapplyThread, nullptr, 0, nullptr);
        if (t) CloseHandle(t); else InterlockedExchange(&g_reapplyRunning, 0);
    }

    void Fallback(const char* why)
    {
        if (InterlockedExchange(&g_failed, 1)) return;
        InterlockedExchange(&g_live, 0);
        char b[200]; sprintf_s(b, "OVERLAY: FALLBACK to real windows (%s)", why); Log(b);
        StartReapply();
    }

    // ---- device side -------------------------------------------------------------------------------
    struct Item { Ov* o; IDirect3DTexture9* tex; ovm::Quad q; float u, v; BYTE ca; int rank; unsigned seq; };
    struct Vtx { float x, y, z, rhw, u, v; };

    void ReleaseAllTextures()
    {
        AcquireSRWLockExclusive(&g_lock);
        for (int i = 0; i < g_ovCount; i++)
        {
            Ov* o = g_ovs[i];
            if (o->tex) { __try { o->tex->Release(); } __except (EXCEPTION_EXECUTE_HANDLER) {} o->tex = nullptr; }
            o->texW = o->texH = 0; o->dirty = o->hasPix;
        }
        ReleaseSRWLockExclusive(&g_lock);
    }

    bool InitDevice(IDirect3DDevice9* dev)
    {
        if (dev->TestCooperativeLevel() != D3D_OK) return false;
        D3DCAPS9 caps;
        if (FAILED(dev->GetDeviceCaps(&caps))) return false;
        InterlockedExchange(&g_maxTex, (LONG)min(caps.MaxTextureWidth, caps.MaxTextureHeight));
        D3DDEVICE_CREATION_PARAMETERS cp;
        g_focus = SUCCEEDED(dev->GetCreationParameters(&cp)) ? cp.hFocusWindow : nullptr;
        if (!g_focus) g_focus = Proxy_GameWnd();
        g_ready = true;
        char b[160]; sprintf_s(b, "OVERLAY: device %p ready (maxTex=%ld, focus=%p) - panels now render in the D3D frame", dev, g_maxTex, g_focus); Log(b);
        if (!g_live) { InterlockedExchange(&g_live, 1); StartReapply(); }
        return true;
    }

    // Uploads dirty pixels, builds the draw list. Returns item count.
    int Collect(IDirect3DDevice9* dev, UINT bbW, UINT bbH, Item* items)
    {
        HWND fw = g_focus ? g_focus : Proxy_GameWnd();
        POINT org = { 0, 0 }; RECT cr = { 0, 0, (LONG)bbW, (LONG)bbH };
        if (fw && IsWindow(fw)) { ClientToScreen(fw, &org); GetClientRect(fw, &cr); }
        float sx = (cr.right > 0) ? (float)bbW / (float)cr.right : 1.0f;
        float sy = (cr.bottom > 0) ? (float)bbH / (float)cr.bottom : 1.0f;
        int n = 0;
        AcquireSRWLockExclusive(&g_lock);
        for (int i = 0; i < g_ovCount && n < MAX_OV; i++)
        {
            Ov* o = g_ovs[i];
            if (!o->hasPix || o->bypass) continue;
            if (!IsWindow(o->h))
            {
                if (o->tex) { o->tex->Release(); o->tex = nullptr; }
                continue;
            }
            if (!IsWindowVisible(o->h) || o->ca == 0) continue;
            RECT wr; if (!GetWindowRect(o->h, &wr)) continue;
            ovm::Quad q = ovm::MapRect(wr.left, wr.top, o->w, o->hgt, org.x, org.y, sx, sy);
            if (!ovm::QuadVisible(q, (float)bbW, (float)bbH)) continue;
            unsigned tw = ovm::Pow2Ceil((unsigned)o->w), th = ovm::Pow2Ceil((unsigned)o->hgt);
            if (o->tex && ((unsigned)o->texW != tw || (unsigned)o->texH != th)) { o->tex->Release(); o->tex = nullptr; }
            if (!o->tex)
            {
                if (FAILED(dev->CreateTexture(tw, th, 1, 0, D3DFMT_A8R8G8B8, D3DPOOL_MANAGED, &o->tex, nullptr)) || !o->tex)
                {
                    o->tex = nullptr; o->bypass = true;   // this one stays a real window
                    char b[96]; sprintf_s(b, "OVERLAY: texture %ux%u failed, window %p stays real", tw, th, o->h); Log(b);
                    StartReapply();
                    continue;
                }
                o->texW = (int)tw; o->texH = (int)th; o->dirty = true;
            }
            if (o->dirty)
            {
                D3DLOCKED_RECT lr;
                if (SUCCEEDED(o->tex->LockRect(0, &lr, nullptr, 0)))
                {
                    for (int y = 0; y < o->hgt; y++)
                    {
                        unsigned* d = (unsigned*)((BYTE*)lr.pBits + (size_t)y * lr.Pitch);
                        memcpy(d, o->pix + (size_t)y * o->w, (size_t)o->w * 4);
                        if ((int)tw > o->w) d[o->w] = 0;   // clamp column: no bilinear bleed from stale texels
                    }
                    if ((int)th > o->hgt) memset((BYTE*)lr.pBits + (size_t)o->hgt * lr.Pitch, 0, (size_t)tw * 4);
                    o->tex->UnlockRect(0);
                    o->dirty = false;
                }
            }
            Item& it = items[n++];
            it.o = o; it.tex = o->tex; it.q = q; it.u = (float)o->w / (float)tw; it.v = (float)o->hgt / (float)th; it.ca = o->ca; it.rank = 0x7fffffff;
        }
        ReleaseSRWLockExclusive(&g_lock);
        if (n > 1)
        {   // real z-order: walk the top-level list from the top (our windows are topmost, so this ends quickly)
            int rem = n, steps = 0;
            for (HWND w = GetTopWindow(nullptr); w && rem && steps < 800; w = GetWindow(w, GW_HWNDNEXT), steps++)
                for (int i = 0; i < n; i++) if (items[i].o->h == w) { items[i].rank = steps; rem--; break; }
            for (int i = 1; i < n; i++)   // insertion sort: bottom (highest rank) first
            {
                Item t = items[i]; int j = i - 1;
                while (j >= 0 && items[j].rank < t.rank) { items[j + 1] = items[j]; j--; }
                items[j + 1] = t;
            }
        }
        return n;
    }

    bool DrawAll(IDirect3DDevice9* dev, IDirect3DSurface9* bb, UINT bbW, UINT bbH, const Item* items, int n)
    {
        IDirect3DSurface9* oldRt = nullptr; IDirect3DStateBlock9* sb = nullptr;
        bool began = false, ok = false, rtChanged = false;
        if (FAILED(dev->CreateStateBlock(D3DSBT_ALL, &sb)) || !sb) return false;
        dev->GetRenderTarget(0, &oldRt);
        if (oldRt != bb) { if (FAILED(dev->SetRenderTarget(0, bb))) goto done; rtChanged = true; }
        if (FAILED(dev->BeginScene())) goto done;   // the game is still inside a scene: skip this frame
        began = true;
        {
            D3DVIEWPORT9 vp = { 0, 0, bbW, bbH, 0.0f, 1.0f };
            dev->SetViewport(&vp);
            dev->SetPixelShader(nullptr); dev->SetVertexShader(nullptr);
            dev->SetFVF(D3DFVF_XYZRHW | D3DFVF_TEX1);
            dev->SetRenderState(D3DRS_ZENABLE, D3DZB_FALSE); dev->SetRenderState(D3DRS_ZWRITEENABLE, FALSE);
            dev->SetRenderState(D3DRS_CULLMODE, D3DCULL_NONE); dev->SetRenderState(D3DRS_FILLMODE, D3DFILL_SOLID);
            dev->SetRenderState(D3DRS_LIGHTING, FALSE); dev->SetRenderState(D3DRS_FOGENABLE, FALSE);
            dev->SetRenderState(D3DRS_SPECULARENABLE, FALSE); dev->SetRenderState(D3DRS_STENCILENABLE, FALSE);
            dev->SetRenderState(D3DRS_SCISSORTESTENABLE, FALSE); dev->SetRenderState(D3DRS_ALPHATESTENABLE, FALSE);
            dev->SetRenderState(D3DRS_COLORWRITEENABLE, 0xF); dev->SetRenderState(D3DRS_SRGBWRITEENABLE, FALSE);
            dev->SetRenderState(D3DRS_CLIPPING, TRUE);
            dev->SetRenderState(D3DRS_ALPHABLENDENABLE, TRUE); dev->SetRenderState(D3DRS_SEPARATEALPHABLENDENABLE, FALSE);
            dev->SetRenderState(D3DRS_BLENDOP, D3DBLENDOP_ADD);
            dev->SetRenderState(D3DRS_SRCBLEND, D3DBLEND_ONE); dev->SetRenderState(D3DRS_DESTBLEND, D3DBLEND_INVSRCALPHA);   // premultiplied
            dev->SetTextureStageState(0, D3DTSS_COLOROP, D3DTOP_MODULATE);
            dev->SetTextureStageState(0, D3DTSS_COLORARG1, D3DTA_TEXTURE); dev->SetTextureStageState(0, D3DTSS_COLORARG2, D3DTA_TFACTOR);
            dev->SetTextureStageState(0, D3DTSS_ALPHAOP, D3DTOP_MODULATE);
            dev->SetTextureStageState(0, D3DTSS_ALPHAARG1, D3DTA_TEXTURE); dev->SetTextureStageState(0, D3DTSS_ALPHAARG2, D3DTA_TFACTOR);
            dev->SetTextureStageState(0, D3DTSS_TEXCOORDINDEX, 0); dev->SetTextureStageState(0, D3DTSS_TEXTURETRANSFORMFLAGS, D3DTTFF_DISABLE);
            dev->SetTextureStageState(1, D3DTSS_COLOROP, D3DTOP_DISABLE); dev->SetTextureStageState(1, D3DTSS_ALPHAOP, D3DTOP_DISABLE);
            dev->SetSamplerState(0, D3DSAMP_MINFILTER, D3DTEXF_LINEAR); dev->SetSamplerState(0, D3DSAMP_MAGFILTER, D3DTEXF_LINEAR);
            dev->SetSamplerState(0, D3DSAMP_MIPFILTER, D3DTEXF_NONE);
            dev->SetSamplerState(0, D3DSAMP_ADDRESSU, D3DTADDRESS_CLAMP); dev->SetSamplerState(0, D3DSAMP_ADDRESSV, D3DTADDRESS_CLAMP);
            dev->SetSamplerState(0, D3DSAMP_SRGBTEXTURE, FALSE);
            ok = true;
            for (int i = 0; i < n; i++)
            {
                const Item& it = items[i];
                Vtx v[4] = {
                    { it.q.x0, it.q.y0, 0.0f, 1.0f, 0.0f, 0.0f },
                    { it.q.x1, it.q.y0, 0.0f, 1.0f, it.u,  0.0f },
                    { it.q.x0, it.q.y1, 0.0f, 1.0f, 0.0f, it.v },
                    { it.q.x1, it.q.y1, 0.0f, 1.0f, it.u,  it.v },
                };
                dev->SetRenderState(D3DRS_TEXTUREFACTOR, D3DCOLOR_ARGB(it.ca, it.ca, it.ca, it.ca));
                dev->SetTexture(0, it.tex);
                if (FAILED(dev->DrawPrimitiveUP(D3DPT_TRIANGLESTRIP, 2, v, sizeof(Vtx)))) ok = false;
            }
        }
    done:
        if (began) dev->EndScene();
        sb->Apply(); sb->Release();
        if (rtChanged && oldRt) dev->SetRenderTarget(0, oldRt);
        if (oldRt) oldRt->Release();
        return ok;
    }

    void Frame(IDirect3DDevice9* dev)
    {
        if (dev != g_dev)
        {
            ReleaseAllTextures();
            g_dev = dev; g_ready = false;
        }
        if (!g_ready && !InitDevice(dev)) return;
        if (g_ovCount == 0) return;
        IDirect3DSurface9* bb = nullptr;
        if (FAILED(dev->GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO, &bb)) || !bb) return;
        D3DSURFACE_DESC bd;
        if (SUCCEEDED(bb->GetDesc(&bd)) && dev->TestCooperativeLevel() == D3D_OK)
        {
            Item items[MAX_OV];
            int n = Collect(dev, bd.Width, bd.Height, items);
            if (n > 0)
            {
                if (DrawAll(dev, bb, bd.Width, bd.Height, items, n)) { g_drawFails = 0; if (!g_loggedFirst) { g_loggedFirst = true; char b[96]; sprintf_s(b, "OVERLAY: first frame drawn, %d panel(s), backbuffer %ux%u", n, bd.Width, bd.Height); Log(b); } }
                else if (++g_drawFails >= 300) Fallback("draw kept failing");
            }
        }
        bb->Release();
    }

    HRESULT STDMETHODCALLTYPE hkPresent(IDirect3DDevice9* self, const RECT* a, const RECT* b, HWND c, const RGNDATA* d)
    {
        if (g_live && !g_failed)
        {
            __try { Frame(self); }
            __except (EXCEPTION_EXECUTE_HANDLER)
            {
                char m[96]; sprintf_s(m, "OVERLAY: exception %d in frame", g_exceptions + 1); Log(m);
                if (++g_exceptions >= 3) Fallback("repeated exceptions");
            }
        }
        else if (!g_live && !g_failed && Enabled() && g_inited)
        {
            __try { Frame(self); }   // first frame(s): InitDevice turns g_live on
            __except (EXCEPTION_EXECUTE_HANDLER) { if (++g_exceptions >= 3) Fallback("init exceptions"); }
        }
        return g_origPresent(self, a, b, c, d);
    }

    bool PatchVt(void** slot, void* hook, void** orig)
    {
        DWORD old;
        if (!VirtualProtect(slot, sizeof(void*), PAGE_READWRITE, &old)) return false;
        *orig = *slot; *slot = hook;
        VirtualProtect(slot, sizeof(void*), old, &old);
        return true;
    }

    HRESULT STDMETHODCALLTYPE hkCreateDevice(IDirect3D9* self, UINT ad, D3DDEVTYPE t, HWND f, DWORD bf, D3DPRESENT_PARAMETERS* pp, IDirect3DDevice9** out)
    {
        HRESULT hr = g_origCreate(self, ad, t, f, bf, pp, out);
        if (SUCCEEDED(hr) && out && *out && Enabled() && !g_failed)
        {
            __try
            {
                void** vt = *(void***)(*out);
                if (!g_hookedDevVt)
                {
                    void* orig = nullptr;
                    if (PatchVt(&vt[17], (void*)hkPresent, &orig)) { g_origPresent = (PFN_Present)orig; g_hookedDevVt = vt; Log("OVERLAY: IDirect3DDevice9::Present hooked"); }
                    else Log("OVERLAY: could not patch device vtable, overlay stays off");
                }
                else if (vt != g_hookedDevVt) Log("OVERLAY: second device type with another vtable, not hooked");
                g_focus = f; g_ready = false;
            }
            __except (EXCEPTION_EXECUTE_HANDLER) { Log("OVERLAY: exception while hooking the device"); }
        }
        return hr;
    }

    bool PatchOwnIat(const char* fn, void* hook, void** orig)
    {
        BYTE* base = (BYTE*)&__ImageBase;
        auto nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
        IMAGE_DATA_DIRECTORY dir = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
        if (!dir.VirtualAddress) return false;
        for (auto d = (IMAGE_IMPORT_DESCRIPTOR*)(base + dir.VirtualAddress); d->Name; d++)
        {
            auto names = (IMAGE_THUNK_DATA*)(base + (d->OriginalFirstThunk ? d->OriginalFirstThunk : d->FirstThunk));
            auto iat = (IMAGE_THUNK_DATA*)(base + d->FirstThunk);
            for (; names->u1.AddressOfData; names++, iat++)
            {
                if (IMAGE_SNAP_BY_ORDINAL(names->u1.Ordinal)) continue;
                auto ibn = (IMAGE_IMPORT_BY_NAME*)(base + names->u1.AddressOfData);
                if (strcmp((const char*)ibn->Name, fn) != 0) continue;
                void** slot = (void**)&iat->u1.Function;
                return PatchVt(slot, hook, orig);
            }
        }
        return false;
    }
}

// DllMain (game host only): hook install, nothing else.
void Ov_Init()
{
    __try
    {
        void* orig = nullptr;
        if (PatchOwnIat("UpdateLayeredWindow", (void*)Ov_ULW, &orig) && orig) { g_realULW = (PFN_ULW)orig; InterlockedExchange(&g_inited, 1); }
        else Log("OVERLAY: UpdateLayeredWindow import not found, overlay off");
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("OVERLAY: exception in Ov_Init"); }
}

// Direct3DCreate9 returned this IDirect3D9 (game host only).
void Ov_OnD3D9Created(void* d3d)
{
    if (!d3d || !g_inited || !Enabled()) return;
    __try
    {
        void** vt = *(void***)d3d;
        if (vt[16] != (void*)hkCreateDevice)
        {
            void* orig = nullptr;
            if (PatchVt(&vt[16], (void*)hkCreateDevice, &orig)) { g_origCreate = (PFN_CreateDevice)orig; Log("OVERLAY: IDirect3D9::CreateDevice hooked"); }
        }
    }
    __except (EXCEPTION_EXECUTE_HANDLER) { Log("OVERLAY: exception hooking IDirect3D9"); }
}
