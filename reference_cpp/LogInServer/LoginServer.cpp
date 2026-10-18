#include "stdafx.h"
#include <sstream>
#include <iostream>
#include <fstream>
#include <windows.h>
#include <time.h>

#include "../shared/Ini.h"
#include "../shared/DateTime.h"

#define BOX_START '#' << uint8(0) << '\n'
#define BOX_END '#'

extern bool g_bRunning;
std::vector<Thread*> g_LogintimerThreads;

void SetConsoleColor(WORD color) {
    HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);
    SetConsoleTextAttribute(hConsole, color);
}

void PrintHeader() {
    HANDLE hConsole = GetStdHandle(STD_OUTPUT_HANDLE);

    // Setting text color to red and printing the header
    SetConsoleColor(FOREGROUND_GREEN + FOREGROUND_RED); // DeaFSezo Renk Sari M�kemmel Ekledim 28.12.2024
    std::cout << "                               ************************************************" << std::endl;
    std::cout << "                               *                                              *" << std::endl;
    std::cout << "                               *     JstKO V1098 & V2383 SERVER FILES         *" << std::endl;
    std::cout << "                               *                                              *" << std::endl;
    std::cout << "                               *                                              *" << std::endl;
    std::cout << "                               *             DISCORD : JstKO                  *" << std::endl;
    std::cout << "                               *				              *" << std::endl;
    std::cout << "                               *                                              *" << std::endl;
    std::cout << "                               ************************************************" << std::endl;

    // Resetting text color to default
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE);
}

LoginServer::LoginServer() { Initialize(); }

void LoginServer::Initialize()
{
    m_sLastVersion = __VERSION;
    m_fpLoginServer = false;
}

bool LoginServer::Startup()
{
    DateTime time;

    PrintHeader();  // Call the function to print the header

    GetInfoFromIni();
    CreateDirectory("Logs", NULL);

    m_fpLoginServer = fopen("./Logs/LoginServer.log", "a");
    if (m_fpLoginServer == nullptr)
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "ERROR: Unable to open log file." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    m_fpUser = fopen(string_format("./Logs/Login_%d_%d_%d.log", time.GetDay(), time.GetMonth(), time.GetYear()).c_str(), "a");
    if (m_fpUser == nullptr)
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "ERROR: Unable to open user log file." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    if (!m_DBProcess.Connect(m_ODBCName, m_ODBCLogin, m_ODBCPwd))
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "HATA: ODBC ile Yapilan ayarlar nedeniyle veritabanina baglanilamiyor." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    SetConsoleColor(FOREGROUND_GREEN);
    std::cout << "Database baglantisi basarili" << std::endl;
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color

    m_DBProcess.LoadKingNotice();

    if (!m_DBProcess.LoadVersionList())
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "HATA: Version listesi yuklenemiyor." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    SetConsoleColor(FOREGROUND_GREEN);
    std::cout << "Servera Giris Versionu: " << GetVersion() << std::endl;
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color

    if (!m_DBProcess.LoadServerList())
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "HATA: Server listesi yuklenemiyor." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    InitPacketHandlers();

    for (int i = 0; i < 10; i++)
    {
        if (!m_socketMgr[i].Listen(m_LoginServerPort + i, MAX_USER))
        {
            SetConsoleColor(FOREGROUND_RED);
            std::cout << "ERROR: Failed to listen on server port." << std::endl;
            SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
            return false;
        }
    }

    for (int i = 0; i < 10; i++)
    {
        m_socketMgr[i].RunServer(true);
    }

    if (!m_GameServerSocketMgr.Listen(m_GameServerSocketPort, 10))
    {
        SetConsoleColor(FOREGROUND_RED);
        std::cout << "ERROR: Failed to listen on server port " << m_GameServerSocketPort << "." << std::endl;
        SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
        return false;
    }

    m_GameServerSocketMgr.RunServer(false);

    g_LogintimerThreads.push_back(new Thread(Timer_UpdateUserCount));
    g_LogintimerThreads.push_back(new Thread(Timer_UpdateKingNotice));

    SetConsoleColor(FOREGROUND_GREEN);
    std::cout << "Login Server Startup" << std::endl;
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
    return true;
}

uint32 LoginServer::Timer_UpdateUserCount(void* lpParam)
{
    while (true)
    {
        g_pMain->UpdateServerList();
        sleep(60 * SECOND);
    }
    return 0;
}

uint32 LoginServer::Timer_UpdateKingNotice(void* lpParam)
{
    while (true)
    {
        sleep(600 * SECOND);

        g_pMain->m_DBProcess.LoadKingNotice();
    }
    return 0;
}

void LoginServer::GetServerList(Packet& result)
{
    Guard lock(m_serverListLock);
    result.append(m_serverListPacket.contents(), m_serverListPacket.size());
}

void LoginServer::UpdateServerList()
{
    m_DBProcess.LoadUserCountList();

    Guard lock(m_serverListLock);
    Packet& result = m_serverListPacket;

    result.clear();
    result << uint8(m_ServerList.GetSize(false));
    foreach_stlmap_nolock(itr, m_ServerList)
    {
        _SERVER_INFO* pServer = itr->second;
        if (pServer == nullptr)
            continue;

        result << pServer->strLanIP << pServer->strServerIP << pServer->strServerName;

        if (pServer->sUserCount <= pServer->sPlayerCap)
            result << pServer->sUserCount;
        else
            result << int16(-1);

        result << pServer->sServerID << pServer->sGroupID << pServer->sPlayerCap << pServer->sFreePlayerCap << uint8(0) << pServer->strScreenType;

        result << pServer->strKarusKingName << pServer->strKarusNotice << pServer->strElMoradKingName << pServer->strElMoradNotice;
    }
}

void LoginServer::GetInfoFromIni()
{
    CIni ini(CONF_LOGIN_SERVER);

    ini.GetString("DOWNLOAD", "URL", "ftp.yoursite.net", m_strFtpUrl, false);
    ini.GetString("DOWNLOAD", "PATH", "/", m_strFilePath, false);

    m_ODBCName = "KO_MAIN";
    m_ODBCLogin = "sa";
    m_ODBCPwd = "E3g8wbc7y";
    m_LoginServerPort = 15100;

    char key[20];

    m_news.Size = 0;
    std::stringstream ss;
    for (int i = 0; i < 3; i++)
    {
        std::string title, message;

        _snprintf(key, sizeof(key), "TITLE_%02d", i);
        ini.GetString("NEWS", key, "", title);
        if (title.empty())
            continue;

        _snprintf(key, sizeof(key), "MESSAGE_%02d", i);
        ini.GetString("NEWS", key, "", message);
        if (message.empty())
            continue;

        size_t oldPos = 0, pos = 0;
        ss << title << BOX_START << message << BOX_END;
    }

    m_news.Size = ss.str().size();
    if (m_news.Size)
        memcpy(&m_news.Content, ss.str().c_str(), m_news.Size);
}

void LoginServer::WriteLogFile(std::string& logMessage)
{
    m_lock.lock();
    fwrite(logMessage.c_str(), logMessage.length(), 1, m_fpLoginServer);
    fflush(m_fpLoginServer);
    m_lock.unlock();
}

void LoginServer::WriteUserLogFile(std::string& logMessage)
{
    m_lock.lock();
    fwrite(logMessage.c_str(), logMessage.length(), 1, m_fpUser);
    fflush(m_fpUser);
    m_lock.unlock();
}

void LoginServer::ReportSQLError(OdbcError* pError)
{
    if (pError == nullptr)
        return;

    std::string errorMessage = string_format(_T("ODBC error occurred.\r\nSource: %s\r\nError: %s\r\nDescription: %s\n"),
        pError->Source.c_str(), pError->ExtendedErrorMessage.c_str(), pError->ErrorMessage.c_str());

    TRACE("%s", errorMessage.c_str());
    WriteLogFile(errorMessage);
    delete pError;
}

LoginServer::~LoginServer()
{
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN); // Yellow
    std::cout << "Waiting for timer threads to exit...";
    foreach(itr, g_LogintimerThreads)
    {
        (*itr)->waitForExit();
        delete (*itr);
    }
    std::cout << " exited." << std::endl;
    Sleep(1 * SECOND);

    m_ServerList.DeleteAllData();

    foreach(itr, m_VersionList)
        delete itr->second;
    m_VersionList.clear();

    if (m_fpLoginServer != nullptr)
        fclose(m_fpLoginServer);

    if (m_fpUser != nullptr)
        fclose(m_fpUser);

    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN); // Yellow
    std::cout << "Shutting down socket system...";

    for (int i = 0; i < 10; i++)
        m_socketMgr[i].Shutdown();

    std::cout << " done." << std::endl;
    SetConsoleColor(FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE); // Reset to default color
    Sleep(1 * SECOND);
}
