#include "stdafx.h"
#include "JvCryption.h"
#include <random>

#define g_private_key 0x1207500120128966

void CJvCryption::Init() { m_tkey = m_public_key ^ g_private_key; }

INLINE time_t getMSTime()
{
#ifdef _WIN32
#if WINVER >= 0x0600
	typedef ULONGLONG(WINAPI* GetTickCount64_t)(void);
	static GetTickCount64_t pGetTickCount64 = nullptr;

	if (!pGetTickCount64)
	{
		HMODULE hModule = LoadLibraryA("KERNEL32.DLL");
		pGetTickCount64 = (GetTickCount64_t)GetProcAddress(hModule, "GetTickCount64");
		if (!pGetTickCount64)
			pGetTickCount64 = (GetTickCount64_t)GetTickCount;
		FreeLibrary(hModule);
	}

	return pGetTickCount64();
#else
	return GetTickCount();
#endif
#else
	struct timeval tv;
	gettimeofday(&tv, nullptr);
	return (tv.tv_sec * SECOND) + (tv.tv_usec / SECOND);
#endif
}

class Guard
{
public:
	Guard(std::recursive_mutex& mutex) : target(mutex) { target.lock(); }
	Guard(std::recursive_mutex* mutex) : target(*mutex) { target.lock(); }
	~Guard() { target.unlock(); }

protected:
	std::recursive_mutex& target;
};

using mt19937 = std::mersenne_twister_engine<unsigned int, 32, 624, 397, 31, 0x9908b0df, 11, 0xffffffff, 7, 0x9d2c5680, 15,
	0xefc60000, 18, 1812433253>;

static std::mt19937 s_randomNumberGenerator;
static std::recursive_mutex s_rngLock;
static bool s_rngSeeded = false;

INLINE void SeedRNG()
{
	if (!s_rngSeeded)
	{
		s_randomNumberGenerator.seed((uint32_t)getMSTime());
		s_rngSeeded = true;
	}
}

int32_t myrand(int32_t min, int32_t max)
{
	Guard lock(s_rngLock);
	SeedRNG();
	if (min > max) std::swap(min, max);
	std::uniform_int_distribution<int32_t> dist(min, max);
	return dist(s_randomNumberGenerator);
}

uint64_t RandUInt64()
{
	Guard lock(s_rngLock);
	SeedRNG();
	std::uniform_int_distribution<uint64_t> dist;
	return dist(s_randomNumberGenerator);
}
uint64_t CJvCryption::GenerateKey()
{
#ifdef USE_CRYPTION
	// because of their sucky encryption method, 0 means it effectively won't be encrypted. 
	// We don't want that happening...
	do
	{
		// NOTE: Debugging
		m_public_key = RandUInt64(); //0xDCE04F8975278163;
	} while (!m_public_key);
#endif
	return m_public_key;
}

void CJvCryption::JvEncryptionFast(int len, uint8_t* datain, uint8_t* dataout)
{
#ifdef USE_CRYPTION
	uint8_t* pkey, lkey, rsk;
	int rkey = 2157;

	pkey = (uint8_t*)&m_tkey;
	lkey = (len * 157) & 0xff;

	for (int i = 0; i < len; i++)
	{
		rsk = (rkey >> 8) & 0xff;
		dataout[i] = ((datain[i] ^ rsk) ^ pkey[(i % 8)]) ^ lkey;
		rkey *= 2171;
	}

	return;
#endif

	dataout = datain;
}

int CJvCryption::JvDecryptionWithCRC32(int len, uint8_t* datain, uint8_t* dataout)
{
#ifdef USE_CRYPTION
	int result;
	JvDecryptionFast(len, datain, dataout);

	if (crc32(dataout, len - 4, -1) == *(uint32_t*)(len - 4 + dataout))
		result = len - 4;
	else
		result = -1;

	return result;
#endif

	dataout = datain;
	return (len - 4);
}
