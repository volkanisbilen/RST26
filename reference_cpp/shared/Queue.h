#pragma once
#include <queue>
#include <thread>
#include <mutex>
#include <condition_variable>

template <typename T>
class Queue
{
public:

	Queue() : running(true)
	{

	}

	T pop()
	{
		std::unique_lock<std::mutex> mlock(mutex_);
		while (queue_.empty())
		{
			if (!running && queue_.empty())
				return nullptr;
			cond_.wait(mlock);
		}
		auto item = queue_.front();
		queue_.pop();
		return item;
	}

	void pop(T& item)
	{
		std::unique_lock<std::mutex> mlock(mutex_);
		while (queue_.empty())
		{
			cond_.wait(mlock);
		}
		item = queue_.front();
		queue_.pop();
	}

	void push(const T& item)
	{
		std::unique_lock<std::mutex> mlock(mutex_);
		queue_.push(item);
		mlock.unlock();
		cond_.notify_one();
	}

	void push(T&& item)
	{
		std::unique_lock<std::mutex> mlock(mutex_);
		queue_.push(std::move(item));
		mlock.unlock();
		cond_.notify_one();
	}

	uint16 size()
	{
		return queue_.size();
	}

	void exit()
	{
		std::unique_lock<std::mutex> mlock(mutex_);
		running = false;
		mlock.unlock();

		cond_.notify_all();
	}

private:
	bool running;
	std::queue<T> queue_;
	std::mutex mutex_;
	std::condition_variable cond_;
};