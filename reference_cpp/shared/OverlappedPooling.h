#pragma once
#include "stdafx.h"
#include "SocketMgr.h"

class OverlappedPooling
{
public:
	OverlappedPooling(Socket* neS, uint32 neLen, OverlappedStruct* neOs) {
		os = neOs;
		s = neS;
		len = neLen;
	};
	OverlappedStruct* os;
	Socket* s;
	uint32 len;
};

