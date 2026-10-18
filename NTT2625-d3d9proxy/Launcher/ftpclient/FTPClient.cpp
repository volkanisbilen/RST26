#include "FTPClient.h"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <sstream>

std::atomic<int> CFTPClient::s_sessionCount{ 0 };

namespace
{
    struct WinInetApi
    {
        HMODULE module = NULL;
        typedef HINTERNET (WINAPI* PFN_InternetOpenA)(LPCSTR, DWORD, LPCSTR, LPCSTR, DWORD);
        typedef HINTERNET (WINAPI* PFN_InternetOpenUrlA)(HINTERNET, LPCSTR, LPCSTR, DWORD, DWORD, DWORD_PTR);
        typedef BOOL (WINAPI* PFN_InternetReadFile)(HINTERNET, LPVOID, DWORD, LPDWORD);
        typedef BOOL (WINAPI* PFN_InternetSetOptionA)(HINTERNET, DWORD, LPVOID, DWORD);
        typedef BOOL (WINAPI* PFN_InternetCloseHandle)(HINTERNET);
        typedef BOOL (WINAPI* PFN_HttpQueryInfoA)(HINTERNET, DWORD, LPVOID, LPDWORD, LPDWORD);
        typedef DWORD (WINAPI* PFN_FtpGetFileSize)(HINTERNET, LPDWORD);

        PFN_InternetOpenA pInternetOpenA = NULL;
        PFN_InternetOpenUrlA pInternetOpenUrlA = NULL;
        PFN_InternetReadFile pInternetReadFile = NULL;
        PFN_InternetSetOptionA pInternetSetOptionA = NULL;
        PFN_InternetCloseHandle pInternetCloseHandle = NULL;
        PFN_HttpQueryInfoA pHttpQueryInfoA = NULL;
        PFN_FtpGetFileSize pFtpGetFileSize = NULL;

        bool Load()
        {
            if (module != NULL)
                return IsReady();

            module = LoadLibraryA("wininet.dll");
            if (module == NULL)
                return false;

            pInternetOpenA = reinterpret_cast<PFN_InternetOpenA>(GetProcAddress(module, "InternetOpenA"));
            pInternetOpenUrlA = reinterpret_cast<PFN_InternetOpenUrlA>(GetProcAddress(module, "InternetOpenUrlA"));
            pInternetReadFile = reinterpret_cast<PFN_InternetReadFile>(GetProcAddress(module, "InternetReadFile"));
            pInternetSetOptionA = reinterpret_cast<PFN_InternetSetOptionA>(GetProcAddress(module, "InternetSetOptionA"));
            pInternetCloseHandle = reinterpret_cast<PFN_InternetCloseHandle>(GetProcAddress(module, "InternetCloseHandle"));
            pHttpQueryInfoA = reinterpret_cast<PFN_HttpQueryInfoA>(GetProcAddress(module, "HttpQueryInfoA"));
            pFtpGetFileSize = reinterpret_cast<PFN_FtpGetFileSize>(GetProcAddress(module, "FtpGetFileSize"));

            return IsReady();
        }

        bool IsReady() const
        {
            return pInternetOpenA != NULL &&
                   pInternetOpenUrlA != NULL &&
                   pInternetReadFile != NULL &&
                   pInternetSetOptionA != NULL &&
                   pInternetCloseHandle != NULL &&
                   pHttpQueryInfoA != NULL &&
                   pFtpGetFileSize != NULL;
        }
    };

    WinInetApi& GetWinInetApi()
    {
        static WinInetApi api;
        return api;
    }
}


static bool StartsWithProtocol(const std::string& value)
{
    std::string upper = value;
    std::transform(upper.begin(), upper.end(), upper.begin(), [](unsigned char c) { return static_cast<char>(::toupper(c)); });
    return upper.rfind("HTTP://", 0) == 0 ||
           upper.rfind("HTTPS://", 0) == 0 ||
           upper.rfind("FTP://", 0) == 0 ||
           upper.rfind("FTPS://", 0) == 0 ||
           upper.rfind("SFTP://", 0) == 0;
}

static std::string TrimSlashesRight(std::string value)
{
    while (!value.empty() && (value.back() == '/' || value.back() == '\\'))
        value.pop_back();
    return value;
}

static std::string TrimSlashesLeft(std::string value)
{
    while (!value.empty() && (value.front() == '/' || value.front() == '\\'))
        value.erase(value.begin());
    return value;
}

CFTPClient::CFTPClient(LogFnCallback logger)
    : m_logger(logger)
{
    ++s_sessionCount;
}

CFTPClient::~CFTPClient()
{
    CleanupSession();
    --s_sessionCount;
}

void CFTPClient::SetProgressFnCallback(void* owner, const ProgressFnCallback& callback)
{
    m_progressOwner = owner;
    m_progressCallback = callback;
    m_progressCallbackSet = static_cast<bool>(m_progressCallback);
}

void CFTPClient::SetProxy(const std::string& proxy)
{
    m_proxy = proxy;
}

const bool CFTPClient::InitSession(const std::string& host,
                                   const unsigned& port,
                                   const std::string& login,
                                   const std::string& password,
                                   const FTP_PROTOCOL& protocol,
                                   const SettingsFlag& settingsFlags)
{
    if (host.empty())
    {
        Log("[FTPClient][Error] Empty host.");
        return false;
    }

    m_server = host;
    m_port = port;
    m_username = login;
    m_password = password;
    m_protocol = protocol;
    m_settingsFlags = static_cast<unsigned char>(settingsFlags);
    m_initialized = true;
    return true;
}

const bool CFTPClient::CleanupSession()
{
    m_initialized = false;
    return true;
}

void CFTPClient::Log(const std::string& message) const
{
    if ((m_settingsFlags & ENABLE_LOG) && m_logger)
        m_logger(message);
}

std::string CFTPClient::BuildURL(const std::string& remoteFile) const
{
    if (StartsWithProtocol(remoteFile))
        return remoteFile;

    std::string base = TrimSlashesRight(m_server);
    std::string path = TrimSlashesLeft(remoteFile);

    if (!StartsWithProtocol(base))
    {
        switch (m_protocol)
        {
        case FTP_PROTOCOL::FTP:
            base = "ftp://" + base;
            break;
        case FTP_PROTOCOL::FTPS:
            base = "ftps://" + base;
            break;
        case FTP_PROTOCOL::SFTP:
            base = "sftp://" + base;
            break;
        case FTP_PROTOCOL::FTPES:
        case FTP_PROTOCOL::HSACSX:
        default:
            base = "http://" + base;
            break;
        }
    }

    if (path.empty())
        return base;

    return base + "/" + path;
}

bool CFTPClient::QueryContentLength(HINTERNET request, DWORD& contentLength) const
{
    contentLength = 0;

    DWORD size = sizeof(contentLength);
    if (GetWinInetApi().pHttpQueryInfoA(request, HTTP_QUERY_CONTENT_LENGTH | HTTP_QUERY_FLAG_NUMBER, &contentLength, &size, NULL))
        return contentLength > 0;

    DWORD high = 0;
    DWORD low = GetWinInetApi().pFtpGetFileSize(request, &high);
    if (low != INVALID_FILE_SIZE || GetLastError() == NO_ERROR)
    {
        if (high == 0)
        {
            contentLength = low;
            return contentLength > 0;
        }
    }

    return false;
}

const bool CFTPClient::Info(const std::string& remoteFile, struct FileInfo& fileInfo) const
{
    fileInfo = FileInfo{};

    if (!m_initialized || remoteFile.empty())
        return false;

    WinInetApi& api = GetWinInetApi();
    if (!api.Load())
    {
        Log("[FTPClient][Error] wininet.dll could not be loaded.");
        return false;
    }

    HINTERNET internet = api.pInternetOpenA("JstKO Launcher", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!internet)
        return false;

    std::string url = BuildURL(remoteFile);
    HINTERNET request = api.pInternetOpenUrlA(internet, url.c_str(), NULL, 0,
        INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE | INTERNET_FLAG_PRAGMA_NOCACHE, 0);

    bool result = false;
    if (request)
    {
        DWORD length = 0;
        if (QueryContentLength(request, length))
        {
            fileInfo.dFileSize = static_cast<double>(length);
            result = true;
        }
        api.pInternetCloseHandle(request);
    }

    api.pInternetCloseHandle(internet);
    return result;
}

const bool CFTPClient::List(const std::string&, std::string& list, bool) const
{
    list.clear();
    return false;
}

const bool CFTPClient::DownloadFile(const std::string& localFile, const std::string& remoteFile) const
{
    if (!m_initialized || localFile.empty() || remoteFile.empty())
        return false;

    std::string url = BuildURL(remoteFile);

    WinInetApi& api = GetWinInetApi();
    if (!api.Load())
    {
        Log("[FTPClient][Error] wininet.dll could not be loaded.");
        return false;
    }

    HINTERNET internet = api.pInternetOpenA("JstKO Launcher", INTERNET_OPEN_TYPE_PRECONFIG, NULL, NULL, 0);
    if (!internet)
    {
        Log("[FTPClient][Error] InternetOpenA failed.");
        return false;
    }

    if (m_timeoutSeconds > 0)
    {
        DWORD timeout = static_cast<DWORD>(m_timeoutSeconds * 1000);
        api.pInternetSetOptionA(internet, INTERNET_OPTION_CONNECT_TIMEOUT, &timeout, sizeof(timeout));
        api.pInternetSetOptionA(internet, INTERNET_OPTION_RECEIVE_TIMEOUT, &timeout, sizeof(timeout));
        api.pInternetSetOptionA(internet, INTERNET_OPTION_SEND_TIMEOUT, &timeout, sizeof(timeout));
    }

    HINTERNET request = api.pInternetOpenUrlA(internet, url.c_str(), NULL, 0,
        INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE | INTERNET_FLAG_PRAGMA_NOCACHE, 0);

    if (!request)
    {
        std::ostringstream os;
        os << "[FTPClient][Error] InternetOpenUrlA failed. URL=" << url << " Error=" << GetLastError();
        Log(os.str());
        api.pInternetCloseHandle(internet);
        return false;
    }

    DWORD contentLength = 0;
    QueryContentLength(request, contentLength);

    std::ofstream output(localFile, std::ios::binary | std::ios::trunc);
    if (!output)
    {
        Log("[FTPClient][Error] Local file cannot be opened for writing: " + localFile);
        api.pInternetCloseHandle(request);
        api.pInternetCloseHandle(internet);
        return false;
    }

    bool ok = true;
    DWORD downloaded = 0;
    BYTE buffer[64 * 1024];

    while (true)
    {
        DWORD bytesRead = 0;
        if (!api.pInternetReadFile(request, buffer, sizeof(buffer), &bytesRead))
        {
            ok = false;
            break;
        }

        if (bytesRead == 0)
            break;

        output.write(reinterpret_cast<const char*>(buffer), bytesRead);
        if (!output)
        {
            ok = false;
            break;
        }

        downloaded += bytesRead;

        if (m_progressCallbackSet)
        {
            double total = contentLength > 0 ? static_cast<double>(contentLength) : static_cast<double>(downloaded);
            m_progressCallback(m_progressOwner, total, static_cast<double>(downloaded), 0.0, 0.0);
        }
    }

    output.close();
    api.pInternetCloseHandle(request);
    api.pInternetCloseHandle(internet);

    if (ok && m_progressCallbackSet)
    {
        double total = contentLength > 0 ? static_cast<double>(contentLength) : static_cast<double>(downloaded);
        m_progressCallback(m_progressOwner, total, static_cast<double>(downloaded), 0.0, 0.0);
    }

    if (!ok)
    {
        std::remove(localFile.c_str());
        Log("[FTPClient][Error] Download failed: " + url);
    }

    return ok;
}
