#include "stdafx.h"

void Socket::AssignToCompletionPort()
{
	CreateIoCompletionPort((HANDLE)m_fd, m_completionPort, (ULONG_PTR)this, 0);
}

void Socket::BurstPush()
{
	if (AcquireSendLock())
		WriteCallback();
}

void Socket::WriteCallback()
{
	m_writeMutex.lock();
	size_t s = writeBuffer.GetContiguousBytes();
	m_writeMutex.unlock();

	if (s)
	{
		m_writeMutex.lock();
		DWORD w_length = 0;
		DWORD flags = 0;

		// attempt to push all the data out in a non-blocking fashion.
		WSABUF buf;
		buf.len = (ULONG)writeBuffer.GetContiguousBytes();
		buf.buf = (char*)writeBuffer.GetBufferStart();
		m_writeMutex.unlock();

		m_writeEvent.Mark();
		m_writeEvent.Reset(SOCKET_IO_EVENT_WRITE_END);

		bool exitd = false;

		int r = WSASend(m_fd, &buf, 1, &w_length, flags, &m_writeEvent.m_overlap, 0);
		if (r == SOCKET_ERROR && WSAGetLastError() != WSA_IO_PENDING)
		{
			exitd = true;
			m_writeEvent.Unmark();
		}

		if (exitd)
		{
			DecSendLock();
			Disconnect(true);
		}
		return;
	}

	DecSendLock();
}

void Socket::SetupReadEvent()
{
	if (IsDeleted() || !IsConnected())
		return;

	Guard lock(m_readMutex);
	DWORD r_length = 0;
	DWORD flags = 0;
	WSABUF buf;
	buf.len = (ULONG)readBuffer.GetSpace();
	buf.buf = (char*)readBuffer.GetBuffer();

	// event that will trigger after data is receieved
	m_readEvent.Mark();
	m_readEvent.Reset(SOCKET_IO_EVENT_READ_COMPLETE);
	if (WSARecv(m_fd, &buf, 1, &r_length, &flags, &m_readEvent.m_overlap, 0) == SOCKET_ERROR)
	{
		int err = WSAGetLastError();
		if (err != WSA_IO_PENDING)
		{
			//printf("SetupReadEvent fail.. %s err %d\n", GetRemoteIP().c_str(), err);
			m_readEvent.Unmark();
			Disconnect();
		}
	}
}
