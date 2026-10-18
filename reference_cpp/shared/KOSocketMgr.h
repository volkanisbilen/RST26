#pragma once

#include <map>
#include "RWLock.h"
#include "SocketMgr.h"
#include "KOSocket.h"
#include "Thread.h"
#include "types.h"
#include <WinSock2.h>

struct _control_check
{

};

typedef std::map<uint16_t, KOSocket*> SessionMap;
typedef std::map<std::string, uint32> SocketControlCheck;

template <class T>
class KOSocketMgr : public SocketMgr
{
public:
	KOSocketMgr<T>() : m_server(nullptr) {}

	virtual void InitSessions(uint16 sTotalSessions);
	virtual bool Listen(uint16 sPort, uint16 sTotalSessions);
	virtual bool Listen(std::string sIPAddress, uint16 sPort, uint16 sTotalSessions);
	uint16	myPort;
	bool isMultiThreaded;
	virtual void OnConnect(Socket *pSock);
	virtual Socket* AssignSocket(SOCKET socket, sockaddr_in m_tempAddress);
	virtual void DisconnectCallback(Socket *pSock);

	void RunServer(bool isMulti)
	{
		isMultiThreaded = isMulti;
		SpawnWorkerThreads(myPort, isMulti);
		GetServer()->run();
	}

	// Send a packet to all active sessions
	void SendAll(Packet * pkt)
	{
		std::lock_guard<std::recursive_mutex> lock(m_lock);
		//LockSessionsRead();
		SessionMap& sessMap = m_activeSessions;
		for (auto itr = sessMap.begin(); itr != sessMap.end(); ++itr)
			itr->second->Send(pkt);
		//UnLockSessionsRead();
	}

	void SendAllCompressed(Packet * result)
	{
		std::lock_guard<std::recursive_mutex> lock(m_lock);
		//LockSessionsRead();
		SessionMap& sessMap = m_activeSessions;
		for (auto itr = sessMap.begin(); itr != sessMap.end(); ++itr)
			itr->second->SendCompressed(result);
		//UnLockSessionsRead();
	}

	ListenSocket<T> * GetServer() { return m_server; }
	INLINE SessionMap & GetIdleSessionMap() { return m_idleSessions; }
	INLINE SessionMap & GetActiveSessionMap() { return m_activeSessions; }
	INLINE std::recursive_mutex& GetLock() { return m_lock; }
	INLINE void LockSessionsRead() { m_sessionLock.r_lock(); };
	INLINE void UnLockSessionsRead() { m_sessionLock.r_unlock(); };
	INLINE void LockSessionsWrite() { m_sessionLock.w_lock(); };
	INLINE void UnLockSessionsWrite() { m_sessionLock.w_unlock(); };

	T * operator[] (uint16 id)
	{
		std::lock_guard<std::recursive_mutex> lock(m_lock);

		T* result = NULL;
		//LockSessionsRead();
		auto itr = m_activeSessions.find(id);
		if (itr != m_activeSessions.end())
			result = static_cast<T*>(itr->second);
		//UnLockSessionsRead();
		return result;
	}

	void Shutdown();
	virtual ~KOSocketMgr();

protected:
	SessionMap m_idleSessions, m_activeSessions, sessizmap;// kanka sana sunucudan tw vereyim benim net yavaþ iþin zorlaþmasýn fark etmz dur az kaldý
	std::recursive_mutex m_lock;
	SocketControlCheck m_ConnectionCheck;
	SRWLock	m_sessionLock;
private:
	ListenSocket<T> * m_server;
};

template <class T>
void KOSocketMgr<T>::InitSessions(uint16 sTotalSessions)
{
	std::lock_guard<std::recursive_mutex> lock(m_lock);
	//LockSessionsWrite();
	for (uint16 i = 0; i < sTotalSessions; i++) m_idleSessions.insert(std::make_pair(i, new T(i, this)));
	//UnLockSessionsWrite();
}

template <class T>
bool KOSocketMgr<T>::Listen(uint16 sPort, uint16 sTotalSessions)
{
	myPort = sPort;
	return Listen("0.0.0.0", sPort, sTotalSessions);
}

template <class T>
bool KOSocketMgr<T>::Listen(std::string sIPAddress, uint16 sPort, uint16 sTotalSessions)
{
	if (m_server != nullptr)
		return false;

	CreateCompletionPort();

	m_server = new ListenSocket<T>(this, sIPAddress.c_str(), sPort);
	if (!m_server->IsOpen())
		return false;

	InitSessions(sTotalSessions);
	return true;
}

template <class T>
Socket* KOSocketMgr<T>::AssignSocket(SOCKET socket, sockaddr_in m_tempAddress)
{
	std::string strIP = (char*)inet_ntoa(m_tempAddress.sin_addr);
	Socket* pSock = nullptr;
	std::lock_guard<std::recursive_mutex> lock(m_lock);
	
		if (strIP != local_ip) 
	{
		auto find = m_ConnectionCheck.find(strIP.c_str());
		if (find != m_ConnectionCheck.end())
		{
			uint32 nCount = find->second;
			if (nCount >= 30) 
			{
				printf("Connection IP %s, Server Access Blocked, Active Connections(%d)\n", strIP.c_str(), nCount);
				return nullptr;
			}
		}
	}

	for (auto itr = m_idleSessions.begin(); itr != m_idleSessions.end(); itr++) {

		if (itr->second->IsDeleted())
			continue;

		m_activeSessions.insert(std::make_pair(itr->first, itr->second));
		pSock = itr->second;
		m_idleSessions.erase(itr);
		pSock->SetFd(socket);

		// Create first connection or count for IP
		if (strIP != local_ip)
		{
			auto find = m_ConnectionCheck.find(strIP.c_str());
			if (find != m_ConnectionCheck.end())
				find->second++;
			else
				m_ConnectionCheck.insert(std::make_pair(strIP.c_str(), 1));
		}

		if (!OpenDesyncPacket && m_activeSessions.size() > 5 && !isMultiThreaded)
			OpenDesyncPacket = true;
		else if (m_activeSessions.size() < 5)
			OpenDesyncPacket = false; // Closed property for Multithread system
		break;
	}
	//UnLockSessionsWrite();
	return pSock;
}

template <class T>
void KOSocketMgr<T>::OnConnect(Socket *pSock)
{
	std::lock_guard<std::recursive_mutex> lock(m_lock);
	//LockSessionsWrite();
	auto itr = m_idleSessions.find(static_cast<KOSocket*>(pSock)->GetSocketID());
	if (itr != m_idleSessions.end()) {
		m_activeSessions.insert(std::make_pair(itr->first, itr->second));
		m_idleSessions.erase(itr);
	}
	//UnLockSessionsWrite();
}

template <class T>
void KOSocketMgr<T>::DisconnectCallback(Socket *pSock)
{
	std::lock_guard<std::recursive_mutex> lock(m_lock);
	//LockSessionsWrite();
	auto itr = m_activeSessions.find(static_cast<T*>(pSock)->GetSocketID());
	if (itr != m_activeSessions.end()) {
		m_idleSessions.insert(std::make_pair(itr->first, itr->second));
		m_activeSessions.erase(itr);

		std::string strIP = static_cast<T*>(pSock)->GetRemoteIP().c_str();
		if (strIP != local_ip) {
			SocketControlCheck::iterator it = m_ConnectionCheck.find(strIP);
			if (it != m_ConnectionCheck.end()) {
				if (it->second)
					it->second--;

				if (it->second == 0)
					m_ConnectionCheck.erase(it);
			}
		}

	}
	//UnLockSessionsWrite();
}

template <class T>
void KOSocketMgr<T>::Shutdown()
{
	if (m_bShutdown)
		return;

	if (m_server != nullptr)
		delete m_server;

	SocketMgr::Shutdown();
}

template <class T>
KOSocketMgr<T>::~KOSocketMgr()
{
	Shutdown();
}