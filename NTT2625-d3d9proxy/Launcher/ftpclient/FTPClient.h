#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif

#include <Windows.h>
#include <wininet.h>
#include <atomic>
#include <ctime>
#include <functional>
#include <fstream>
#include <mutex>
#include <string>
#include <vector>

// x86/x64 uyumlu, libcurl/WinINet import library import bagimliligi olmayan hafif indirme istemcisi.
// Eski CFTPClient arayuzunun Launcher tarafinda kullanilan bolumlerini korur.
class CFTPClient
{
public:
    typedef std::function<int(void*, double, double, double, double)> ProgressFnCallback;
    typedef std::function<void(const std::string&)> LogFnCallback;

    struct FileInfo
    {
        time_t tFileMTime = 0;
        double dFileSize = 0.0;
    };

    enum SettingsFlag
    {
        NO_FLAGS = 0x00,
        ENABLE_LOG = 0x01,
        ENABLE_SSH = 0x02,
        ALL_FLAGS = 0xFF
    };

    enum class FTP_PROTOCOL : unsigned char
    {
        FTP,
        FTPS,
        FTPES,
        SFTP,
        HSACSX // Projede HTTP patch indirme icin kullaniliyor.
    };

    explicit CFTPClient(LogFnCallback logger);
    virtual ~CFTPClient();

    CFTPClient(const CFTPClient&) = delete;
    CFTPClient& operator=(const CFTPClient&) = delete;

    void SetProgressFnCallback(void* owner, const ProgressFnCallback& callback);
    void SetProxy(const std::string& proxy);
    void SetTimeout(const int& timeoutSeconds) { m_timeoutSeconds = timeoutSeconds; }
    void SetActive(const bool& enable) { m_active = enable; }
    void SetNoSignal(const bool& noSignal) { m_noSignal = noSignal; }

    inline const int GetTimeout() const { return m_timeoutSeconds; }
    inline const unsigned GetPort() const { return m_port; }
    inline const bool GetActive() const { return m_active; }
    inline const bool GetNoSignal() const { return m_noSignal; }
    inline const std::string& GetURL() const { return m_server; }
    inline const std::string& GetUsername() const { return m_username; }
    inline const std::string& GetPassword() const { return m_password; }
    inline const unsigned char GetSettingsFlags() const { return m_settingsFlags; }
    inline const FTP_PROTOCOL GetProtocol() const { return m_protocol; }

    const bool InitSession(const std::string& host,
                           const unsigned& port,
                           const std::string& login,
                           const std::string& password,
                           const FTP_PROTOCOL& protocol = FTP_PROTOCOL::FTP,
                           const SettingsFlag& settingsFlags = ALL_FLAGS);
    virtual const bool CleanupSession();
    static int GetCurlSessionCount() { return s_sessionCount.load(); }

    const bool Info(const std::string& remoteFile, struct FileInfo& fileInfo) const;
    const bool List(const std::string& remoteFolder, std::string& list, bool onlyNames = true) const;
    const bool DownloadFile(const std::string& localFile, const std::string& remoteFile) const;

    // Eski header ile kaynak uyumlulugu icin tutulan stub metodlar.
    const bool CreateDir(const std::string&) const { return false; }
    const bool RemoveDir(const std::string&) const { return false; }
    const bool RemoveFile(const std::string&) const { return false; }
    const bool DownloadWildcard(const std::string&, const std::string&) const { return false; }
    const bool UploadFile(const std::string&, const std::string&, const bool& = false) const { return false; }
    void SetSSLCertFile(const std::string& path) { m_sslCertFile = path; }
    const std::string& GetSSLCertFile() const { return m_sslCertFile; }
    void SetSSLKeyFile(const std::string& path) { m_sslKeyFile = path; }
    const std::string& GetSSLKeyFile() const { return m_sslKeyFile; }
    void SetSSLKeyPassword(const std::string& password) { m_sslKeyPassword = password; }
    const std::string& GetSSLKeyPwd() const { return m_sslKeyPassword; }

private:
    std::string BuildURL(const std::string& remoteFile) const;
    void Log(const std::string& message) const;
    bool QueryContentLength(HINTERNET request, DWORD& contentLength) const;

    LogFnCallback m_logger;
    ProgressFnCallback m_progressCallback;
    void* m_progressOwner = nullptr;
    bool m_progressCallbackSet = false;

    std::string m_server;
    std::string m_username;
    std::string m_password;
    std::string m_proxy;
    std::string m_sslCertFile;
    std::string m_sslKeyFile;
    std::string m_sslKeyPassword;

    unsigned m_port = 0;
    int m_timeoutSeconds = 0;
    bool m_active = false;
    bool m_noSignal = false;
    bool m_initialized = false;
    unsigned char m_settingsFlags = ALL_FLAGS;
    FTP_PROTOCOL m_protocol = FTP_PROTOCOL::FTP;

    static std::atomic<int> s_sessionCount;
};
