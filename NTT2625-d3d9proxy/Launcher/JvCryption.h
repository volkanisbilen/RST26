#pragma once

#define USE_CRYPTION

extern uint32_t crc32(const uint8_t* s, uint32_t len, uint32_t startVal);

class CJvCryption
{
private:
	uint64_t m_public_key, m_tkey;

public:
	CJvCryption() : m_public_key(0) {}

	INLINE uint64_t GetPublicKey() const {
		return m_public_key;
	}

	INLINE void SetPublicKey(uint64_t key) {
		m_public_key = key;
	}

	uint64_t GenerateKey();

	void Init();

	void JvEncryptionFast(int len, uint8_t* datain, uint8_t* dataout);
	INLINE void JvDecryptionFast(int len, uint8_t* datain, uint8_t* dataout) {
		JvEncryptionFast(len, datain, dataout);
	}

	int JvDecryptionWithCRC32(int len, uint8_t* datain, uint8_t* dataout);
};
