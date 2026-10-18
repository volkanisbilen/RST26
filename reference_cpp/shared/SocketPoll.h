#pragma once

#include "OverlappedPooling.h"

class SocketPoll
{
public:
	// Startup the poll threads
	static void Startup();

	// Add to the queue and notify threads of activity.
	static void AddRequest(OverlappedPooling * op);
	static void AddRequestWrite(OverlappedPooling * op);

	// Main thread procedure
	static uint32 THREADCALL ThreadProc(void * lpParam);
	static uint32 THREADCALL ThreadProcWrite(void * lpParam);

	// Shutdown threads.
	static void Shutdown();

};

