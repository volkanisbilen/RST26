#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#define _WINSOCK_DEPRECATED_NO_WARNINGS

#include <winsock2.h>
#include <ws2tcpip.h>
#include <Windows.h>
#include <ShellAPI.h>
#include <string>
#include <vector>
#include <stdlib.h>
#include <stdio.h>
#include <fstream>
#include <streambuf>
#include <mutex>
#include "Packet.h"
#include "xorstr.h"

#pragma comment (lib, "AdvApi32.lib")

#define DEFAULT_BUFLEN 512
