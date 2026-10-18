#include "stdafx.h"
#include "DynamicWinsock.h"

namespace
{
    HMODULE g_hWs2 = NULL;

    typedef int       (WSAAPI* PFN_WSAStartup)(WORD, LPWSADATA);
    typedef int       (WSAAPI* PFN_WSACleanup)(void);
    typedef int       (WSAAPI* PFN_WSAGetLastError)(void);
    typedef SOCKET    (WSAAPI* PFN_socket)(int, int, int);
    typedef int       (WSAAPI* PFN_connect)(SOCKET, const struct sockaddr*, int);
    typedef int       (WSAAPI* PFN_closesocket)(SOCKET);
    typedef int       (WSAAPI* PFN_ioctlsocket)(SOCKET, long, u_long*);
    typedef int       (WSAAPI* PFN_recv)(SOCKET, char*, int, int);
    typedef int       (WSAAPI* PFN_send)(SOCKET, const char*, int, int);
    typedef u_short   (WSAAPI* PFN_htons)(u_short);
    typedef u_short   (WSAAPI* PFN_ntohs)(u_short);
    typedef unsigned long (WSAAPI* PFN_inet_addr)(const char*);
    typedef struct hostent* (WSAAPI* PFN_gethostbyname)(const char*);
    typedef int       (WSAAPI* PFN_setsockopt)(SOCKET, int, int, const char*, int);
    typedef int       (WSAAPI* PFN_WSAAsyncSelect)(SOCKET, HWND, unsigned int, long);

    PFN_WSAStartup      pWSAStartup = NULL;
    PFN_WSACleanup      pWSACleanup = NULL;
    PFN_WSAGetLastError pWSAGetLastError = NULL;
    PFN_socket          psocket = NULL;
    PFN_connect         pconnect = NULL;
    PFN_closesocket     pclosesocket = NULL;
    PFN_ioctlsocket     pioctlsocket = NULL;
    PFN_recv            precv = NULL;
    PFN_send            psend = NULL;
    PFN_htons           phtons = NULL;
    PFN_ntohs           pntohs = NULL;
    PFN_inet_addr       pinet_addr = NULL;
    PFN_gethostbyname   pgethostbyname = NULL;
    PFN_setsockopt      psetsockopt = NULL;
    PFN_WSAAsyncSelect  pWSAAsyncSelect = NULL;

    template <typename T>
    bool LoadProc(T& out, const char* name)
    {
        out = reinterpret_cast<T>(GetProcAddress(g_hWs2, name));
        return out != NULL;
    }

    u_short LocalByteSwap16(u_short value)
    {
        return static_cast<u_short>(((value & 0x00FF) << 8) | ((value & 0xFF00) >> 8));
    }
}

bool JstWSAEnsureLoaded()
{
    if (g_hWs2 != NULL)
        return true;

    g_hWs2 = LoadLibraryA("ws2_32.dll");
    if (g_hWs2 == NULL)
        return false;

    bool ok = true;
    ok = LoadProc(pWSAStartup, "WSAStartup") && ok;
    ok = LoadProc(pWSACleanup, "WSACleanup") && ok;
    ok = LoadProc(pWSAGetLastError, "WSAGetLastError") && ok;
    ok = LoadProc(psocket, "socket") && ok;
    ok = LoadProc(pconnect, "connect") && ok;
    ok = LoadProc(pclosesocket, "closesocket") && ok;
    ok = LoadProc(pioctlsocket, "ioctlsocket") && ok;
    ok = LoadProc(precv, "recv") && ok;
    ok = LoadProc(psend, "send") && ok;
    ok = LoadProc(phtons, "htons") && ok;
    ok = LoadProc(pntohs, "ntohs") && ok;
    ok = LoadProc(pinet_addr, "inet_addr") && ok;
    ok = LoadProc(pgethostbyname, "gethostbyname") && ok;
    ok = LoadProc(psetsockopt, "setsockopt") && ok;
    ok = LoadProc(pWSAAsyncSelect, "WSAAsyncSelect") && ok;

    if (!ok)
    {
        FreeLibrary(g_hWs2);
        g_hWs2 = NULL;
        return false;
    }

    return true;
}

void JstWSARelease()
{
    if (g_hWs2 != NULL)
    {
        FreeLibrary(g_hWs2);
        g_hWs2 = NULL;
    }

    pWSAStartup = NULL;
    pWSACleanup = NULL;
    pWSAGetLastError = NULL;
    psocket = NULL;
    pconnect = NULL;
    pclosesocket = NULL;
    pioctlsocket = NULL;
    precv = NULL;
    psend = NULL;
    phtons = NULL;
    pntohs = NULL;
    pinet_addr = NULL;
    pgethostbyname = NULL;
    psetsockopt = NULL;
    pWSAAsyncSelect = NULL;
}

int WSAAPI JstWSAStartup(WORD wVersionRequested, LPWSADATA lpWSAData)
{
    if (!JstWSAEnsureLoaded() || pWSAStartup == NULL)
        return WSASYSNOTREADY;
    return pWSAStartup(wVersionRequested, lpWSAData);
}

int WSAAPI JstWSACleanup(void)
{
    if (!JstWSAEnsureLoaded() || pWSACleanup == NULL)
        return SOCKET_ERROR;
    return pWSACleanup();
}

int WSAAPI JstWSAGetLastError(void)
{
    if (JstWSAEnsureLoaded() && pWSAGetLastError != NULL)
        return pWSAGetLastError();
    return static_cast<int>(GetLastError());
}

SOCKET WSAAPI JstSocket(int af, int type, int protocol)
{
    if (!JstWSAEnsureLoaded() || psocket == NULL)
        return INVALID_SOCKET;
    return psocket(af, type, protocol);
}

int WSAAPI JstConnect(SOCKET s, const struct sockaddr* name, int namelen)
{
    if (!JstWSAEnsureLoaded() || pconnect == NULL)
        return SOCKET_ERROR;
    return pconnect(s, name, namelen);
}

int WSAAPI JstCloseSocket(SOCKET s)
{
    if (!JstWSAEnsureLoaded() || pclosesocket == NULL)
        return SOCKET_ERROR;
    return pclosesocket(s);
}

int WSAAPI JstIoctlSocket(SOCKET s, long cmd, u_long* argp)
{
    if (!JstWSAEnsureLoaded() || pioctlsocket == NULL)
        return SOCKET_ERROR;
    return pioctlsocket(s, cmd, argp);
}

int WSAAPI JstRecv(SOCKET s, char* buf, int len, int flags)
{
    if (!JstWSAEnsureLoaded() || precv == NULL)
        return SOCKET_ERROR;
    return precv(s, buf, len, flags);
}

int WSAAPI JstSend(SOCKET s, const char* buf, int len, int flags)
{
    if (!JstWSAEnsureLoaded() || psend == NULL)
        return SOCKET_ERROR;
    return psend(s, buf, len, flags);
}

u_short WSAAPI JstHtons(u_short hostshort)
{
    if (JstWSAEnsureLoaded() && phtons != NULL)
        return phtons(hostshort);
    return LocalByteSwap16(hostshort);
}

u_short WSAAPI JstNtohs(u_short netshort)
{
    if (JstWSAEnsureLoaded() && pntohs != NULL)
        return pntohs(netshort);
    return LocalByteSwap16(netshort);
}

unsigned long WSAAPI JstInetAddr(const char* cp)
{
    if (!JstWSAEnsureLoaded() || pinet_addr == NULL)
        return INADDR_NONE;
    return pinet_addr(cp);
}

struct hostent* WSAAPI JstGetHostByName(const char* name)
{
    if (!JstWSAEnsureLoaded() || pgethostbyname == NULL)
        return NULL;
    return pgethostbyname(name);
}

int WSAAPI JstSetSockOpt(SOCKET s, int level, int optname, const char* optval, int optlen)
{
    if (!JstWSAEnsureLoaded() || psetsockopt == NULL)
        return SOCKET_ERROR;
    return psetsockopt(s, level, optname, optval, optlen);
}

int WSAAPI JstWSAAsyncSelect(SOCKET s, HWND hWnd, unsigned int wMsg, long lEvent)
{
    if (!JstWSAEnsureLoaded() || pWSAAsyncSelect == NULL)
        return SOCKET_ERROR;
    return pWSAAsyncSelect(s, hWnd, wMsg, lEvent);
}
