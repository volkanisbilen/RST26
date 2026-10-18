#include "stdafx.h"
#include "../shared/Condition.h"
#include "SocketMgr.h"
#include "SocketPoll.h"


using std::string;
static std::queue<OverlappedPooling *> _queue;
static std::queue<OverlappedPooling *> _queueWrite;
static bool _running = true;
static std::recursive_mutex _lock;
static std::recursive_mutex _lockWrite;

static Condition s_hEvent;
static Condition s_hEventWrite;
static Thread * s_thread;
static Thread * s_threadWrite;

void SocketPoll::Startup()
{
	s_thread = new Thread(ThreadProc, (void *)1);
	s_threadWrite = new Thread(ThreadProcWrite, (void *)2);
}

void SocketPoll::AddRequest(OverlappedPooling * op)
{
	_lock.lock();
	_queue.push(op);
	_lock.unlock();
	s_hEvent.Signal();
}

void SocketPoll::AddRequestWrite(OverlappedPooling * op)
{
	_lockWrite.lock();
	_queueWrite.push(op);
	_lockWrite.unlock();
	s_hEventWrite.Signal();
}


uint32 THREADCALL SocketPoll::ThreadProcWrite(void * lpParam)
{
	while (_running)
	{
		OverlappedPooling *op = nullptr;

		// Pull the next packet from the shared queue
		_lockWrite.lock();
		if (_queueWrite.size())
		{

			op = _queueWrite.front();
			_queueWrite.pop();
		}
		_lockWrite.unlock();
		// If there's no more packets to handle, wait until there are.
		if (op == nullptr)
		{
			// If we're shutting down, don't bother waiting for more (there are no more).
			if (!_running)
				break;

			s_hEventWrite.Wait();
			continue;
		}


		ophandlers[op->os->m_event](op->s, op->len);

		// Free the packet.
		delete op;
	}

	return 0;
}

void SocketPoll::Shutdown()
{
	_running = false;

	s_hEvent.Broadcast();
	s_hEventWrite.Broadcast();

	s_thread->waitForExit();
	s_threadWrite->waitForExit();

	delete s_thread;
	delete s_threadWrite;
}

uint32 THREADCALL SocketPoll::ThreadProc(void * lpParam)
{
	while (_running)
	{
		OverlappedPooling *op = nullptr;

		// Pull the next packet from the shared queue
		_lock.lock();
		if (_queue.size())
		{

			op = _queue.front();
			_queue.pop();
		}
		_lock.unlock();
		// If there's no more packets to handle, wait until there are.
		if (op == nullptr)
		{
			// If we're shutting down, don't bother waiting for more (there are no more).
			if (!_running)
				break;

			s_hEvent.Wait();
			continue;
		}


		ophandlers[op->os->m_event](op->s, op->len);

		// Free the packet.
		delete op;
	}

	return 0;
}