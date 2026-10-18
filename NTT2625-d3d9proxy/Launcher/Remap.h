#pragma once

#include <Windows.h>
#include <iostream>
#include <winternl.h>

typedef enum _SECTION_INHERIT
{
	ViewShare = 1,
	ViewUnmap = 2

} SECTION_INHERIT;

#define STATUS_SUCCESS 0x00000000

#pragma comment(lib, "ntdll.lib")

EXTERN_C NTSTATUS NTAPI ZwCreateSection(PHANDLE, ACCESS_MASK, POBJECT_ATTRIBUTES, PLARGE_INTEGER, ULONG, ULONG, HANDLE);
EXTERN_C NTSTATUS NTAPI ZwMapViewOfSection(HANDLE, HANDLE, PVOID*, ULONG_PTR, SIZE_T, PLARGE_INTEGER, PSIZE_T, SECTION_INHERIT, ULONG, ULONG);
EXTERN_C NTSTATUS NTAPI ZwUnmapViewOfSection(HANDLE, PVOID);

class Remap
{
private:
	inline static bool RemapViewOfSection(HANDLE ProcessHandle, PVOID BaseAddress, SIZE_T RegionSize, DWORD NewProtection, PVOID CopyBuffer)
	{
		SIZE_T numberOfBytesRead = 0;
		if (ReadProcessMemory(ProcessHandle, BaseAddress, CopyBuffer, RegionSize, &numberOfBytesRead) == FALSE)
		{
			return false;
		}

		HANDLE hSection = NULL;
		LARGE_INTEGER sectionMaxSize = {};
		sectionMaxSize.QuadPart = RegionSize;

		NTSTATUS R = ZwCreateSection(&hSection, SECTION_ALL_ACCESS, NULL, &sectionMaxSize, PAGE_EXECUTE_READWRITE, SEC_COMMIT, NULL);

		if (R != STATUS_SUCCESS)
		{
			return false;
		}

		NTSTATUS R2 = ZwUnmapViewOfSection(ProcessHandle, BaseAddress);

		if (R2 != STATUS_SUCCESS)
		{
			return false;
		}

		PVOID viewBase = BaseAddress;
		LARGE_INTEGER sectionOffset = {};
		SIZE_T viewSize = 0;

		NTSTATUS R3 = ZwMapViewOfSection(hSection, ProcessHandle, &viewBase, 0, RegionSize, &sectionOffset, &viewSize, ViewUnmap, 0, NewProtection);

		if (R3 != STATUS_SUCCESS)
		{
			return false;
		}

		SIZE_T numberOfBytesWritten2 = 0;

		if (WriteProcessMemory(ProcessHandle, viewBase, CopyBuffer, viewSize, &numberOfBytesWritten2) == FALSE)
		{
			return false;
		}

		if (CloseHandle(hSection) == FALSE)
		{
			return false;
		}

		return true;
	}

public:
	inline static bool PatchSection(HANDLE hProcess, PVOID regionBase, SIZE_T regionSize, DWORD newProtection)
	{
		PVOID EmptyAlloc = VirtualAlloc(NULL, regionSize, MEM_COMMIT | MEM_RESERVE, newProtection);

		if (EmptyAlloc == NULL)
		{
			return false;
		}

		if (RemapViewOfSection(hProcess, regionBase, regionSize, newProtection, EmptyAlloc) == false)
		{
			return false;
		}

		if (VirtualFree(EmptyAlloc, NULL, MEM_RELEASE) == FALSE)
		{
			return false;
		}

		return true;
	}
};

