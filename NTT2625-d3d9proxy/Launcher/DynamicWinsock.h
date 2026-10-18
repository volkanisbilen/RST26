#pragma once

#include <winsock2.h>
#include <Windows.h>

// WS2 import library import library gerektirmeden ws2_32.dll uzerinden calisan socket wrapper.
// Boylece x86/x64 Release build sırasında LNK1181 'WS2 import library' hatası alınmaz.
bool JstWSAEnsureLoaded();
void JstWSARelease();

int WSAAPI JstWSAStartup(WORD wVersionRequested, LPWSADATA lpWSAData);
int WSAAPI JstWSACleanup(void);
int WSAAPI JstWSAGetLastError(void);
SOCKET WSAAPI JstSocket(int af, int type, int protocol);
int WSAAPI JstConnect(SOCKET s, const struct sockaddr* name, int namelen);
int WSAAPI JstCloseSocket(SOCKET s);
int WSAAPI JstIoctlSocket(SOCKET s, long cmd, u_long* argp);
int WSAAPI JstRecv(SOCKET s, char* buf, int len, int flags);
int WSAAPI JstSend(SOCKET s, const char* buf, int len, int flags);
u_short WSAAPI JstHtons(u_short hostshort);
u_short WSAAPI JstNtohs(u_short netshort);
unsigned long WSAAPI JstInetAddr(const char* cp);
struct hostent* WSAAPI JstGetHostByName(const char* name);
int WSAAPI JstSetSockOpt(SOCKET s, int level, int optname, const char* optval, int optlen);
int WSAAPI JstWSAAsyncSelect(SOCKET s, HWND hWnd, unsigned int wMsg, long lEvent);
