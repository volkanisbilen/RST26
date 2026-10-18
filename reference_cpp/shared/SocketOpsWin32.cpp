#include "stdafx.h"

struct Connections {
public:
	ULONGLONG l_time, b_time;
	uint16 c_count;
};

std::recursive_mutex m_iplistlock;
std::map<std::string, Connections> m_iplist;
namespace SocketOps
{
	// Create file descriptor for socket i/o operations.
	SOCKET CreateTCPFileDescriptor()
	{
		// create a socket for use with overlapped i/o.
		return ::WSASocket(AF_INET, SOCK_STREAM, 0, 0, 0, WSA_FLAG_OVERLAPPED);
	}

	// Disable blocking send/recv calls.
	bool Nonblocking(SOCKET fd)
	{
		u_long arg = 1;
		return (::ioctlsocket(fd, FIONBIO, &arg) == 0);
	}

	// Disable blocking send/recv calls.
	bool Blocking(SOCKET fd)
	{
		u_long arg = 0;
		return (ioctlsocket(fd, FIONBIO, &arg) == 0);
	}

	// Disable nagle buffering algorithm
	bool DisableBuffering(SOCKET fd)
	{
		uint32 arg = 1;
		return (setsockopt(fd, 0x6, TCP_NODELAY, (const char*)&arg, sizeof(arg)) == 0);
	}

	// Enable nagle buffering algorithm
	bool EnableBuffering(SOCKET fd)
	{
		uint32 arg = 0;
		return (setsockopt(fd, 0x6, TCP_NODELAY, (const char*)&arg, sizeof(arg)) == 0);
	}

	// Closes a socket fully.
	void CloseSocket(SOCKET fd, std::string adress)
	{
		shutdown(fd, SD_BOTH);
		closesocket(fd);
	}

	bool CheckedConnect(SOCKET fd, std::string adress)
	{
		if (adress.empty())
			return false;

		if (adress == "5.9.173.118" || adress == "5.9.173.118")
			return true;

		Guard lock(m_iplistlock);

		ULONGLONG time = GetTickCount64();

		auto it = m_iplist.find(adress);
		if (it == m_iplist.end())
		{
			Connections p;
			p.l_time = time;
			p.b_time = 0;
			p.c_count = 0;
			m_iplist.emplace(std::make_pair(adress, p));
			return true;
		}

		if (it->second.b_time > time)
			return false;

		if (time - it->second.l_time < 650)
		{
			if (++it->second.c_count > 4)
				it->second.b_time = time + (20 * SECOND);
			else
				it->second.b_time = time + (10 * SECOND);
			it->second.l_time = time;
			return false;
		}

		it->second.l_time = time;
		it->second.c_count = 0;
		return true;
	}
}
