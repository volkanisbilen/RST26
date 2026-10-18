#include "stdafx.h"
#include "SocketMgr.h"
#include "future"
#include "SocketPoll.h"

bool SocketMgr::s_bRunningCleanupThread = true;
std::recursive_mutex SocketMgr::s_disconnectionQueueLock;
std::vector<Socket*> SocketMgr::s_disconnectionQueue;

Thread SocketMgr::s_cleanupThread;
Atomic<uint32> SocketMgr::s_refCounter;

uint32 THREADCALL SocketCleanupThread(void* lpParam)
{
	while (SocketMgr::s_bRunningCleanupThread)
	{
		SocketMgr::s_disconnectionQueueLock.lock();
		for (size_t i = 0; i < SocketMgr::s_disconnectionQueue.size(); i++)
		{
			Socket* pSock = SocketMgr::s_disconnectionQueue[i];
			if (pSock->GetSocketMgr() && !pSock->IsDeleted())
			{
				pSock->GetSocketMgr()->DisconnectCallback(pSock);
				SocketMgr::s_disconnectionQueue.erase(SocketMgr::s_disconnectionQueue.begin() + i);
			}
		}
		SocketMgr::s_disconnectionQueueLock.unlock();
		sleep(100);
	}
	return 0;
}

SocketMgr::SocketMgr() : m_bWorkerThreadsActive(false),
m_bShutdown(false)
{
	static bool bRefCounterInitialised = false;
	if (!bRefCounterInitialised)
	{
		s_refCounter = 0;
		bRefCounterInitialised = true;
	}
	OpenDesyncPacket = false;
	IncRef();
	Initialise();
}

void SocketMgr::SpawnWorkerThreads(uint16 myPort, bool MultiT)
{
	m_threadPort = myPort;

	if (m_bWorkerThreadsActive)
		return;

	m_bWorkerThreadsActive = true;
	//m_thread = new Thread(SocketWorkerThread, this);

	// Create worker threads based on the number of processors available on the
	// system. Create two worker threads for each processor
	 //Xtreme eski port 15001 Yeni 15572
	if (m_threadPort == 15572)
	{
		if (MultiT)
		{
			SYSTEM_INFO SystemInfo;
			// Determine how many processors are on the system
			GetSystemInfo(&SystemInfo);
			m_threadCount = (int)SystemInfo.dwNumberOfProcessors * 2;
		}
		else
		{
			m_threadCount = 6;
			SocketPoll::Startup();
		}
	}
	 //Xtreme eski port 15001 yeni 15572
	if (m_threadPort != 15572)
		m_threadCount = 1;

	for (int i = 0; i < m_threadCount; i++)
	{
		m_thread.push_back(new Thread(SocketWorkerThread, this));
	}

	/*printf("%d threads created for socket system (%d)\n", m_threadCount, myPort);*/

	if (!s_cleanupThread.isStarted())
		s_cleanupThread.start(SocketCleanupThread);
}


uint32 THREADCALL SocketMgr::SocketWorkerThread(void* lpParam)
{
	SocketMgr* socketMgr = (SocketMgr*)lpParam;
	HANDLE cp = socketMgr->GetCompletionPort();
	DWORD len;
	Socket* s = nullptr;
	OverlappedStruct* ov = nullptr;
	LPOVERLAPPED ol_ptr;

	while (socketMgr->m_bWorkerThreadsActive)
	{

#ifndef _WIN64
		if (!GetQueuedCompletionStatus(cp, &len, (LPDWORD)&s, &ol_ptr, INFINITE))
#else
		if (!GetQueuedCompletionStatus(cp, &len, (PULONG_PTR)&s, &ol_ptr, INFINITE))
#endif
		{
			if (s != nullptr && !s->IsOffCharacter())
			{
				s->Disconnect();
			}
			continue;
		}

		ov = CONTAINING_RECORD(ol_ptr, OverlappedStruct, m_overlap);

		if (ov->m_event == SOCKET_IO_THREAD_SHUTDOWN)
		{
			delete ov;
			return 0;
		}

		if (socketMgr->OpenDesyncPacket)
		{
			OverlappedPooling* op = new OverlappedPooling(s, len, ov);

			if (ov->m_event == SOCKET_IO_EVENT_WRITE_END)
				SocketPoll::AddRequestWrite(op);
			else if (ov->m_event < NUM_SOCKET_IO_EVENTS)
				SocketPoll::AddRequest(op);

		}
		else if (ov->m_event < NUM_SOCKET_IO_EVENTS)
			ophandlers[ov->m_event](s, len);

		//std::async(std::launch::async, ophandlers[ov->m_event], s, len);
		//std::thread::id this_id = std::this_thread::get_id();
		//printf("Thread %llu is worked..\n", this_id);
	}

	return 0;
}



void SocketMgr::Initialise()
{
	m_completionPort = nullptr;
}

void SocketMgr::CreateCompletionPort()
{
	SetCompletionPort(CreateIoCompletionPort(INVALID_HANDLE_VALUE, nullptr, (ULONG_PTR)0, 0));
}

void SocketMgr::SetupWinsock()
{
	WSADATA wsaData;
	WSAStartup(MAKEWORD(2, 0), &wsaData);
}

void HandleReadComplete(Socket* s, uint32 len)
{
	if (s->IsDeleted())
		return;


	s->m_readEvent.Unmark();
	if (len)
	{
		s->GetReadBuffer().IncrementWritten(len);
		s->OnRead();
		s->SetupReadEvent();
	}
	else
	{
		s->Disconnect();
	}
}

void HandleWriteComplete(Socket* s, uint32 len)
{
	if (s->IsDeleted())
		return;

	s->m_writeEvent.Unmark();
	s->BurstBegin();
	s->GetWriteBuffer().Remove(len);
	if (s->GetWriteBuffer().GetContiguousBytes() > 0)
		s->WriteCallback();
	else
		s->DecSendLock();
	s->BurstEnd();
}

void HandleShutdown(Socket* s, uint32 len) {}

void SocketMgr::OnConnect(Socket* pSock) {}
void SocketMgr::DisconnectCallback(Socket* pSock) {}
void SocketMgr::OnDisconnect(Socket* pSock)
{
	Guard lock(s_disconnectionQueueLock);
	s_disconnectionQueue.push_back(pSock);
}

void SocketMgr::ShutdownThreads()
{
	OverlappedStruct* ov = new OverlappedStruct(SOCKET_IO_THREAD_SHUTDOWN);
	PostQueuedCompletionStatus(m_completionPort, 0, (ULONG_PTR)0, &ov->m_overlap);

	m_bWorkerThreadsActive = false;

	foreach(itr, m_thread)
		(*itr)->waitForExit();

	for (int i = 0; i < m_threadCount; i++)
		delete m_thread[m_threadCount - i - 1];
}

void SocketMgr::Shutdown()
{
	if (m_bShutdown)
		return;

	ShutdownThreads();

	DecRef();
	m_bShutdown = true;
}

void SocketMgr::SetupSockets()
{
	SetupWinsock();
}

void SocketMgr::CleanupSockets()
{
	if (s_cleanupThread.isStarted())
	{
		s_bRunningCleanupThread = false;
		s_cleanupThread.waitForExit();
	}

	WSACleanup();
}

SocketMgr::~SocketMgr()
{
	Shutdown();
}
