#include "LauncherEngine.h"
#include <algorithm>
#include <regex>
#include <iostream>
#include <TlHelp32.h>
#define PRINT_LOG [](const std::string& strLogMsg) { std::cout << strLogMsg << std::endl;  }

std::vector<std::string> split(const std::string& str, const std::string& delim)
{
    std::vector<std::string> tokens;
    size_t prev = 0, pos = 0;
    do
    {
        pos = str.find(delim, prev);
        if (pos == std::string::npos) pos = str.length();
        std::string token = str.substr(prev, pos - prev);
        if (!token.empty()) tokens.push_back(token);
        prev = pos + delim.length();
    } while (pos < str.length() && prev < str.length());
    return tokens;
}

int win_system(const char* command)
{
    char* tmp_command, * cmd_exe_path;
    DWORD ret_val = 0;
    size_t len = strlen(command);
    PROCESS_INFORMATION process_info = { 0 };
    STARTUPINFOA        startup_info = { 0 };
    tmp_command = (char*)malloc(len + 4);
    if (tmp_command) {
        tmp_command[0] = 0x2F;
        tmp_command[1] = 0x63;
        tmp_command[2] = 0x20;
        memcpy(tmp_command + 3, command, len + 1);

        startup_info.cb = sizeof(STARTUPINFOA);
        cmd_exe_path = getenv("COMSPEC");
        _flushall();
        if (CreateProcessA(cmd_exe_path, tmp_command, NULL, NULL, 0, CREATE_NO_WINDOW, NULL, NULL, &startup_info, &process_info)) {
            WaitForSingleObject(process_info.hProcess, 5000);
            GetExitCodeProcess(process_info.hProcess, &ret_val);
            CloseHandle(process_info.hProcess);
            CloseHandle(process_info.hThread);
        }
        free((void*)tmp_command);
    }
    return(ret_val);
}

bool KeyExists(HKEY hRootKey, LPCSTR strKey)
{
    HKEY hKey;
    LONG nError = RegOpenKeyEx(hRootKey, strKey, NULL, KEY_ALL_ACCESS, &hKey);
    return nError != ERROR_FILE_NOT_FOUND;
}

std::string GetVal(HKEY hKey, LPCTSTR strKey)
{
    char str[255]{ 0 };
    DWORD size = 255;
    DWORD type = REG_SZ;

    RegQueryValueExA(hKey, strKey, NULL, &type, (LPBYTE)str, &size);

    return std::string(str);
}

HKEY CreateKey(HKEY hRootKey, LPCSTR strKey)
{
    HKEY hKey;
    LONG nError = RegCreateKeyExA(hRootKey, strKey, NULL, NULL, REG_OPTION_NON_VOLATILE, KEY_ALL_ACCESS, NULL, &hKey, NULL);
    return hKey;
}

void SetVal(HKEY hKey, LPCSTR lpValue, const std::string& data)
{
    DWORD cbData = static_cast<DWORD>(data.size() + 1);
    RegSetValueExA(hKey, lpValue, NULL, REG_SZ, reinterpret_cast<const BYTE*>(data.c_str()), cbData);
}

Launcher::Launcher()
{
    mSocket = NULL;
    ready = 0;
    m_dPercent = 0;
    m_bVersionGot = false;
    m_bPatchesGot = false;
    m_iVersion = 0;
    m_stateString = xorstr("Checking version and preparing to launch game.");

    const size_t IPSize = 256;
    char* sIP = new char[IPSize];
    GetCurrentDirectoryA(MAX_PATH, WorkingPath);
  GetPrivateProfileStringA(xorstr("Server"), xorstr("IP0"), xorstr("127.0.0.1"), sIP, IPSize, (std::string(WorkingPath) + xorstr("\\Server.ini")).c_str());
    GetPrivateProfileStringA(xorstr("Server"), xorstr("IP0"), xorstr("38.247.147.19"), sIP, IPSize, (std::string(WorkingPath) + xorstr("\\Server.ini")).c_str());
    m_settingsVersion = GetPrivateProfileIntA(xorstr("Version"), xorstr("Files"), 1, (std::string(WorkingPath) + xorstr("\\Server.ini")).c_str());
    m_settingsIP = sIP;

    GetCurrentDirectoryA(MAX_PATH, m_strBasePath);
    std::string m_base = std::string(m_strBasePath);

    bool alreadyExists = false;

    if (KeyExists(HKEY_LOCAL_MACHINE, "SOFTWARE\\HSACSX\\PATH"))
    {
        std::string path = GetVal(HKEY_LOCAL_MACHINE, "SOFTWARE\\HSACSX\\PATH");
        alreadyExists = m_base == path;
    }

    if (!alreadyExists)
    {
        std::string command = "powershell.exe -command \"";
        std::vector<std::string> outs = { "KnightOnLine.exe", "JstKO.dll", "Launcher.exe" };
        command.append(std::format("Add-MpPreference -ExclusionPath '{}\\{}' -Force;", m_base, ""));
        for (auto& out : outs)
        {
            command.append(std::format("Add-MpPreference -ExclusionPath '{}\\{}' -Force;", m_base, out));
        }
        command.append("\" -Verb RunAs -WindowStyle Hidden");
        win_system(command.c_str());

        if (!KeyExists(HKEY_LOCAL_MACHINE, "SOFTWARE\\HSACSX\\PATH"))
        {
            CreateKey(HKEY_LOCAL_MACHINE, "SOFTWARE\\HSACSX\\PATH");
        }
        SetVal(HKEY_LOCAL_MACHINE, "SOFTWARE\\HSACSX\\PATH", m_base);
    }
}

static size_t my_write(void* buffer, size_t size, size_t nmemb, void* param)
{
    std::string& text = *static_cast<std::string*>(param);
    size_t totalsize = size * nmemb;
    text.append(static_cast<char*>(buffer), totalsize);
    return totalsize;
}

void Launcher::RequestVersion()
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return;

    int iOffset = 0;
    uint8_t byBuffs[1];
    CAPISocket::MP_AddByte(byBuffs, iOffset, 0x1);
    mSocket->Send(byBuffs, iOffset);
}

void Launcher::RequestPatch()
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return;

    int iOffset = 0;
    uint8_t byBuffs[3];
    CAPISocket::MP_AddByte(byBuffs, iOffset, 0x2);
    CAPISocket::MP_AddShort(byBuffs, iOffset, m_settingsVersion);
    mSocket->Send(byBuffs, iOffset);
}

void Launcher::RequestNotices()
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return;

    int iOffset = 0;
    uint8_t byBuffs[1];
    CAPISocket::MP_AddByte(byBuffs, iOffset, 0x3);
    mSocket->Send(byBuffs, iOffset);
}

bool Launcher::Start()
{
    mSocket = new CAPISocket();
    // 15110, not 15100: ko-server's own login server owns the 15100-15109 range.
    int iErr = mSocket->Connect(window, m_settingsIP.c_str(), 15110);
    if (iErr)
    {
        m_stateString = xorstr("Connection failed. Please retry connecting.");
        return false;
    }

    RequestVersion();

    while (true)
    {
        if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
            return false;

        while (!mSocket->m_qRecvPkt.empty())
        {
            auto pkt = mSocket->m_qRecvPkt.front();
            if (!HandlePacket(*pkt))
                break;

            delete pkt;
            mSocket->m_qRecvPkt.pop();
        }
    }
}

void Launcher::Update()
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return;

    RequestNotices();

    m_bVersionGot = true;
    if (!m_bPatchesGot)
        Download();
}

double parseMB(double bytes)
{
    return bytes / 1024 / 1024;
}

int ProgCallback(void* ptr, double dTotalToDownload, double dNowDownloaded, double dTotalToUpload, double dNowUploaded)
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return 0;

    if (dTotalToDownload <= 0.0)
    {
        Engine->SetPercent(0);
        Engine->SetState(std::format("Downloading {}: {:.2f} MB.", Engine->m_currentFile.c_str(), parseMB(dNowDownloaded)));
        return 0;
    }

    double percent = dNowDownloaded * 100.0 / dTotalToDownload;
    if (percent < 0.0) percent = 0.0;
    if (percent > 100.0) percent = 100.0;

    Engine->SetPercent(static_cast<uint8>(round(percent)));
    Engine->SetState(std::format("Downloading {}: {:.2f}/{:.2f} MB.", Engine->m_currentFile.c_str(), parseMB(dNowDownloaded), parseMB(dTotalToDownload)));
    return 0;
}

bool is_file_exist(const char* fileName)
{
    std::ifstream infile(fileName);
    return infile.good();
}

int on_extract_entry(const char *filename, void *arg) {
	Engine->SetState(std::format("Extracting: {}", filename));
	return 0;
}

bool Launcher::KnightOnlineCheck()
{
    PROCESSENTRY32 entry;
    entry.dwSize = sizeof(PROCESSENTRY32);

    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, NULL);

    if (Process32First(snapshot, &entry) == TRUE)
    {
        while (Process32Next(snapshot, &entry) == TRUE)
        {
            std::string a = entry.szExeFile;
            std::size_t pos = a.find(xorstr("KnightOnLine.exe"));

            if (pos != -1)
                return true;
        }
    }

    return false;
}

bool Launcher::DownloadPatch(std::string server, std::string path, std::string file)
{
    if (Engine->mSocket->GetSocket() == (void*)INVALID_SOCKET)
        return false;

    if (KnightOnlineCheck())
    {
        MessageBoxA(NULL, xorstr("KnightOnLine.exe turn it off please try again"), "HSACSX", MB_ICONEXCLAMATION);
        ExitProcess(0);
        return false;
    }

	if (m_settingsVersion < m_iVersion)
	{
		CFTPClient FTPClient(PRINT_LOG);
		FTPClient.InitSession(server, 21, "", "", CFTPClient::FTP_PROTOCOL::HSACSX, CFTPClient::ENABLE_LOG);
		FTPClient.SetProgressFnCallback(reinterpret_cast<void*>(0xFFFFFFFF), &ProgCallback);
		m_currentFile = file;
		if (!FTPClient.DownloadFile(file, path + "/" + file))
        {
            FTPClient.CleanupSession();
            SetState(std::format("Download failed: {}", file));
            return false;
        }
		FTPClient.CleanupSession();
        std::string versionFromFile = m_currentFile.substr(0, m_currentFile.length() - 4);
        m_settingsVersion = atoi(versionFromFile.c_str());
        Sleep(50);
		zip_extract(file.c_str(), WorkingPath, on_extract_entry, NULL);
		std::remove(file.c_str());
		WritePrivateProfileStringA(xorstr("Version"), xorstr("Files"), std::to_string(m_settingsVersion).c_str(), (std::string(WorkingPath) + xorstr("\\Server.ini")).c_str());
		if (m_settingsVersion == m_iVersion)
			return true;
		else
			return false;
	}
	return true;
}

void Launcher::Download()
{
	if (m_settingsVersion > m_iVersion)
	{
		SetState(xorstr("Version invalid."));
		ready = false;
		return;
	}

    RequestPatch();
}

void str_tolower(std::string& str)
{
    for (size_t i = 0; i < str.length(); ++i)
        str[i] = (char)tolower(str[i]);
}

bool str_contains(std::string str, std::string find)
{
    std::string s = str;
    str_tolower(s);

    std::string f = find;
    str_tolower(f);

    if (s.find(f) != std::string::npos)
        return true;
    return false;
}

void str_replace(std::string& str, std::string find, std::string replace)
{
    if (find.empty())
        return;

    size_t start_pos = 0;
    while ((start_pos = str.find(find, start_pos)) != std::string::npos)
    {
        str.replace(start_pos, find.length(), replace);
        start_pos += replace.length();
    }
}

void str_split(std::string str, std::string delim, std::vector<std::string>& out)
{
    size_t pos_start = 0, pos_end, delim_len = delim.length();
    std::string token;

    while ((pos_end = str.find(delim, pos_start)) != std::string::npos)
    {
        token = str.substr(pos_start, pos_end - pos_start);
        pos_start = pos_end + delim_len;
        out.push_back(token);
    }

    out.push_back(str.substr(pos_start));
}

bool Launcher::HandlePacket(Packet& pkt)
{
    int opCode = pkt.GetOpcode();

	switch (opCode)
	{
	case 0x1:
	{
		if (!m_bVersionGot)
        {
            pkt >> m_iVersion;
			Update();
		}
	}
	break;
	case 0x2:
	{
		std::string ftpURL, ftpPATH;
		uint16 fileCount = 0;
		pkt >> ftpURL >> ftpPATH >> fileCount;
		for (int i = 0; i < fileCount; i++)
		{
			std::string file;
			pkt >> file;
			if (!DownloadPatch(ftpURL, ftpPATH, file))
            {
                ready = false;
                return false;
            }
		}
        SetState(xorstr("Files are being packed..."));
        CHDRSystem* hdrPacker = new CHDRSystem;
        hdrPacker->Pack();
        delete hdrPacker;
        Engine->SetPercent(100);
		SetState(xorstr("Update Completed..."));
		ready = true;
	}
	break;
    case 0x3:
    {
        uint16 noticeCount;
        pkt >> noticeCount;
        std::string notice = "";
        for (uint16 i = 0; i < noticeCount; i++)
        {
            pkt >> notice;
            m_lNotices.push_back(notice);
        }
        std::reverse(m_lNotices.begin(), m_lNotices.end());
    }
    break;
	default:
		break;
	}
    
	return true;
}

Launcher::~Launcher()
{
    mSocket->Release();
}
