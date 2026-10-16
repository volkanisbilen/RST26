// Launcher-side patch pulling: fetches ko-admin's /jstko_patch/manifest.json over HTTP(S) via
// WinHTTP, compares each listed file's SHA-256 against the local copy, and downloads whatever is
// missing or different before the launcher starts the game (see launcher.cpp's WorkThread).
//
// Best-effort by design: any network/parse failure just skips patching for this run (a VPS
// hiccup must never block play) and is written to launcher_patch.log next to the launcher.
//
// Server address comes from JstKO\patch_server.ini ([Patch] Url=http://host:port), default
// http://84.247.183.23:8090 (the VPS, see package_patch.py) -- editable without a rebuild if the
// host or port ever changes.
//
// Build: adds winhttp.lib to launcher.cpp's link line (build_new.bat / the Launcher project),
// alongside bcrypt.lib already used elsewhere in this repo for hashing.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <winhttp.h>
#include <bcrypt.h>
#include <stdio.h>
#include <string>
#include <vector>
#pragma comment(lib, "winhttp.lib")
#pragma comment(lib, "bcrypt.lib")

namespace
{
    struct Entry
    {
        std::string path;   // forward slashes, e.g. "Data/Item_Org_us.tbl"
        std::string sha256; // lowercase hex, 64 chars
        size_t size = 0;
    };

    std::wstring ExeDir()
    {
        wchar_t p[MAX_PATH];
        GetModuleFileNameW(nullptr, p, MAX_PATH);
        std::wstring s(p);
        size_t i = s.find_last_of(L"\\/");
        return i == std::wstring::npos ? L"." : s.substr(0, i);
    }

    void LogLine(const std::string& line)
    {
        std::wstring p = ExeDir() + L"\\launcher_patch.log";
        FILE* f = nullptr;
        if (_wfopen_s(&f, p.c_str(), L"a") == 0 && f)
        {
            SYSTEMTIME t;
            GetLocalTime(&t);
            fprintf(f, "%04d-%02d-%02d %02d:%02d:%02d  %s\n", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, line.c_str());
            fclose(f);
        }
    }

    std::wstring ToWide(const std::string& s)
    {
        if (s.empty()) return L"";
        int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, 0);
        MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), &w[0], n);
        return w;
    }

    bool ReadServerUrl(std::wstring& host, INTERNET_PORT& port, bool& https)
    {
        wchar_t ini[MAX_PATH];
        swprintf_s(ini, L"%s\\JstKO\\patch_server.ini", ExeDir().c_str());
        wchar_t url[256];
        GetPrivateProfileStringW(L"Patch", L"Url", L"http://84.247.183.23:8090", url, 256, ini);
        std::wstring u = url;
        https = u.rfind(L"https://", 0) == 0;
        size_t skip = https ? 8 : (u.rfind(L"http://", 0) == 0 ? 7 : 0);
        std::wstring rest = u.substr(skip);
        size_t slash = rest.find(L'/');
        if (slash != std::wstring::npos) rest = rest.substr(0, slash); // strip any accidental path
        size_t colon = rest.find(L':');
        if (colon == std::wstring::npos)
        {
            host = rest;
            port = https ? 443 : 80;
        }
        else
        {
            host = rest.substr(0, colon);
            port = (INTERNET_PORT)_wtoi(rest.substr(colon + 1).c_str());
        }
        return !host.empty();
    }

    // Raw GET; returns the response body, or empty + *ok=false on any failure.
    std::string HttpGet(const std::wstring& host, INTERNET_PORT port, bool https, const std::wstring& path, bool* ok)
    {
        *ok = false;
        std::string out;
        HINTERNET hSession = WinHttpOpen(L"JstKOLauncher/1.0", WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);
        if (!hSession) return out;
        WinHttpSetTimeouts(hSession, 5000, 5000, 20000, 20000);
        HINTERNET hConnect = WinHttpConnect(hSession, host.c_str(), port, 0);
        if (hConnect)
        {
            DWORD flags = https ? WINHTTP_FLAG_SECURE : 0;
            HINTERNET hRequest = WinHttpOpenRequest(hConnect, L"GET", path.c_str(), nullptr,
                WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, flags);
            if (hRequest)
            {
                if (WinHttpSendRequest(hRequest, WINHTTP_NO_ADDITIONAL_HEADERS, 0, WINHTTP_NO_REQUEST_DATA, 0, 0, 0) &&
                    WinHttpReceiveResponse(hRequest, nullptr))
                {
                    DWORD status = 0, sz = sizeof(status);
                    WinHttpQueryHeaders(hRequest, WINHTTP_QUERY_FLAG_NUMBER | WINHTTP_QUERY_STATUS_CODE,
                        WINHTTP_HEADER_NAME_BY_INDEX, &status, &sz, WINHTTP_NO_HEADER_INDEX);
                    if (status == 200)
                    {
                        *ok = true;
                        for (;;)
                        {
                            DWORD avail = 0;
                            if (!WinHttpQueryDataAvailable(hRequest, &avail) || avail == 0) break;
                            std::vector<char> buf(avail);
                            DWORD read = 0;
                            if (!WinHttpReadData(hRequest, buf.data(), avail, &read) || read == 0) break;
                            out.append(buf.data(), read);
                        }
                    }
                }
                WinHttpCloseHandle(hRequest);
            }
            WinHttpCloseHandle(hConnect);
        }
        WinHttpCloseHandle(hSession);
        return out;
    }

    // Minimal parser for exactly what package_patch.py emits: a flat JSON array of
    // {"path": "...", "sha256": "...", "size": N} objects, no nesting, no escaped characters in
    // any string (true for every path/hash this project produces). Not a general JSON parser.
    std::vector<Entry> ParseManifest(const std::string& body)
    {
        std::vector<Entry> out;
        size_t i = 0;
        auto skipWs = [&]() { while (i < body.size() && (unsigned char)body[i] <= ' ') i++; };
        auto readString = [&](std::string& s) -> bool
        {
            skipWs();
            if (i >= body.size() || body[i] != '"') return false;
            i++;
            size_t start = i;
            while (i < body.size() && body[i] != '"') i++;
            if (i >= body.size()) return false;
            s = body.substr(start, i - start);
            i++;
            return true;
        };
        while (i < body.size() && body[i] != '[') i++;
        if (i >= body.size()) return out;
        i++;
        for (;;)
        {
            skipWs();
            if (i >= body.size() || body[i] == ']') break;
            if (body[i] == ',') { i++; continue; }
            if (body[i] != '{') break;
            i++;
            Entry e{};
            for (;;)
            {
                skipWs();
                if (i >= body.size() || body[i] == '}') { i++; break; }
                if (body[i] == ',') { i++; continue; }
                std::string key;
                if (!readString(key)) return out;
                skipWs();
                if (i >= body.size() || body[i] != ':') return out;
                i++;
                skipWs();
                if (key == "path") { if (!readString(e.path)) return out; }
                else if (key == "sha256") { if (!readString(e.sha256)) return out; }
                else if (key == "size")
                {
                    size_t start = i;
                    while (i < body.size() && (isdigit((unsigned char)body[i]))) i++;
                    e.size = (size_t)_atoi64(body.substr(start, i - start).c_str());
                }
                else
                {
                    // unknown field: skip a string or a bare token so we stay in sync
                    if (i < body.size() && body[i] == '"') { std::string tmp; readString(tmp); }
                    else { while (i < body.size() && body[i] != ',' && body[i] != '}') i++; }
                }
            }
            if (!e.path.empty() && e.sha256.size() == 64) out.push_back(e);
        }
        return out;
    }

    bool Sha256File(const std::wstring& path, std::string& hexOut)
    {
        HANDLE f = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (f == INVALID_HANDLE_VALUE) return false;
        bool ok = false;
        BCRYPT_ALG_HANDLE alg = nullptr;
        if (BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, nullptr, 0) == 0)
        {
            BCRYPT_HASH_HANDLE hash = nullptr;
            if (BCryptCreateHash(alg, &hash, nullptr, 0, nullptr, 0, 0) == 0)
            {
                static BYTE buf[1 << 16];
                DWORD got = 0;
                bool readOk = true;
                while (ReadFile(f, buf, sizeof(buf), &got, nullptr) && got > 0)
                {
                    if (BCryptHashData(hash, buf, got, 0) != 0) { readOk = false; break; }
                }
                if (readOk)
                {
                    BYTE out[32];
                    if (BCryptFinishHash(hash, out, 32, 0) == 0)
                    {
                        char hex[65];
                        for (int k = 0; k < 32; k++) sprintf_s(hex + k * 2, 3, "%02x", out[k]);
                        hexOut.assign(hex, 64);
                        ok = true;
                    }
                }
                BCryptDestroyHash(hash);
            }
            BCryptCloseAlgorithmProvider(alg, 0);
        }
        CloseHandle(f);
        return ok;
    }

    void MakeDirsFor(const std::wstring& filePath)
    {
        size_t pos = 0;
        while ((pos = filePath.find(L'\\', pos + 1)) != std::wstring::npos)
            CreateDirectoryW(filePath.substr(0, pos).c_str(), nullptr);
    }

    bool DownloadFile(const std::wstring& host, INTERNET_PORT port, bool https, const Entry& e, const std::wstring& dest)
    {
        bool ok = false;
        std::wstring urlPath = L"/jstko_patch/files/" + ToWide(e.path);
        std::string body = HttpGet(host, port, https, urlPath, &ok);
        if (!ok || body.size() != e.size) return false;
        std::wstring tmp = dest + L".part";
        MakeDirsFor(tmp);
        HANDLE f = CreateFileW(tmp.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (f == INVALID_HANDLE_VALUE) return false;
        DWORD written = 0;
        bool wrote = WriteFile(f, body.data(), (DWORD)body.size(), &written, nullptr) && written == body.size();
        CloseHandle(f);
        if (!wrote) { DeleteFileW(tmp.c_str()); return false; }
        std::string gotHash;
        if (!Sha256File(tmp, gotHash) || gotHash != e.sha256) { DeleteFileW(tmp.c_str()); return false; }
        if (!MoveFileExW(tmp.c_str(), dest.c_str(), MOVEFILE_REPLACE_EXISTING)) { DeleteFileW(tmp.c_str()); return false; }
        return true;
    }
}

// Runs once; onStatus (may be null) gets short progress strings for the splash window.
// Always returns -- a down/unreachable patch server just means nothing gets updated this run.
void LauncherPatch_Apply(void (*onStatus)(const wchar_t* text))
{
    std::wstring host;
    INTERNET_PORT port;
    bool https;
    if (!ReadServerUrl(host, port, https)) return;

    if (onStatus) onStatus(L"Guncellemeler kontrol ediliyor...");
    bool ok = false;
    std::string body = HttpGet(host, port, https, L"/jstko_patch/manifest.json", &ok);
    if (!ok)
    {
        LogLine("manifest fetch failed, skipping patch this run");
        return;
    }
    std::vector<Entry> entries = ParseManifest(body);
    if (entries.empty()) return;

    std::wstring dir = ExeDir();
    int updated = 0, failed = 0;
    for (size_t n = 0; n < entries.size(); n++)
    {
        const Entry& e = entries[n];
        std::wstring rel = ToWide(e.path);
        for (auto& c : rel) if (c == L'/') c = L'\\';
        std::wstring full = dir + L"\\" + rel;

        std::string localHash;
        bool upToDate = Sha256File(full, localHash) && localHash == e.sha256;
        if (upToDate) continue;

        if (onStatus)
        {
            wchar_t msg[160];
            swprintf_s(msg, L"Guncelleniyor %zu/%zu: %s", n + 1, entries.size(), rel.c_str());
            onStatus(msg);
        }
        if (DownloadFile(host, port, https, e, full))
        {
            updated++;
        }
        else
        {
            failed++;
            char buf[600];
            sprintf_s(buf, "failed to update %s", e.path.c_str());
            LogLine(buf);
        }
    }
    if (updated || failed)
    {
        char buf[128];
        sprintf_s(buf, "patch run: %d updated, %d failed, %d already current", updated, failed, (int)entries.size() - updated - failed);
        LogLine(buf);
    }
}
