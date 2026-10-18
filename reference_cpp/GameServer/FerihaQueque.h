#pragma once

#include <condition_variable>
#include <deque>
class AdiniFerihaKoydum
{
public:
	AdiniFerihaKoydum();
	virtual ~AdiniFerihaKoydum();

	void Shutdown(int type);

	void tKnightLogger();
	void tDatabase();

	void AddRequest(int type ,Packet* pkt);

	void BeginSynchronized(int type);
	void EndSynchronized(int type);
	uint32 Wait(int type,time_t timeout);
	uint32 Wait(int type);
	void Signal(int type);
	void Broadcast(int type);

	std::queue<Packet*> _queue[(int)dbreqtype::COUNT];

	bool _running[(int)dbreqtype::COUNT];
	std::recursive_mutex _lock[(int)dbreqtype::COUNT];
	std::thread mthread[(int)dbreqtype::COUNT];

	int m_nLockCount[(int)dbreqtype::COUNT];
	std::condition_variable m_condition[(int)dbreqtype::COUNT];
	std::mutex m_conditionlock[(int)dbreqtype::COUNT];
	
	void Initialize();
};
