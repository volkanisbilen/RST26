[CmdletBinding()]
param(
    [ValidateSet('Local', 'Vps')]
    [string]$Target = 'Local',
    [string]$VpsHost = '84.247.183.23',
    [string]$SshUser = 'root',
    [int]$TunnelPort = 15432,
    [string]$DatabaseUrl = ''
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$previousDatabaseUrl = $env:DATABASE_URL
$tunnel = $null
$utf8 = New-Object System.Text.UTF8Encoding($false)
[Console]::InputEncoding = $utf8
[Console]::OutputEncoding = $utf8
$OutputEncoding = $utf8

Push-Location $projectRoot
try {
    if ($Target -eq 'Vps') {
        if ($VpsHost -notmatch '^[a-zA-Z0-9.-]+$' -or $SshUser -notmatch '^[a-zA-Z0-9_-]+$') {
            throw 'Geçersiz SSH sunucu veya kullanıcı adı.'
        }
        if ($TunnelPort -lt 1024 -or $TunnelPort -gt 65535) { throw 'Geçersiz tünel portu.' }
        if (Get-NetTCPConnection -LocalPort $TunnelPort -State Listen -ErrorAction SilentlyContinue) {
            throw "Tünel portu $TunnelPort kullanımda. Başka bir TunnelPort seçin."
        }
        if (-not $DatabaseUrl) {
            Write-Host "VPS veritabanı ayarı okunuyor: $VpsHost. SSH şifresi istenirse bu pencereye girin."
            $remoteDatabaseUrl = & ssh.exe -T -o ConnectTimeout=15 -o PreferredAuthentications=password -o PubkeyAuthentication=no "${SshUser}@${VpsHost}" "grep '^DATABASE_URL=' /etc/ko-server.env | sed 's/^DATABASE_URL=//'"
            if ($LASTEXITCODE -ne 0 -or -not $remoteDatabaseUrl) {
                throw 'VPS DATABASE_URL okunamadı.'
            }
            $DatabaseUrl = ($remoteDatabaseUrl | Select-Object -First 1).Trim()
        }
        Write-Host "VPS bağlantısı: $VpsHost. SSH şifresi istenirse bu pencereye girin."
        $tunnel = Start-Process -FilePath 'ssh.exe' -NoNewWindow -PassThru -ArgumentList @(
            '-N', '-T', '-o', 'ExitOnForwardFailure=yes', '-o', 'ConnectTimeout=15',
            '-o', 'ServerAliveInterval=30', '-o', 'ServerAliveCountMax=3',
            '-L', "127.0.0.1:${TunnelPort}:127.0.0.1:5432", "${SshUser}@${VpsHost}"
        )
        $deadline = [DateTime]::UtcNow.AddMinutes(2)
        while (-not (Get-NetTCPConnection -LocalPort $TunnelPort -State Listen -ErrorAction SilentlyContinue)) {
            if ($tunnel.HasExited) { throw 'SSH bağlantısı kurulamadı.' }
            if ([DateTime]::UtcNow -gt $deadline) { throw 'SSH bağlantısı zaman aşımına uğradı.' }
            Start-Sleep -Milliseconds 250
        }
        $uri = New-Object System.UriBuilder($DatabaseUrl)
        $uri.Host = '127.0.0.1'
        $uri.Port = $TunnelPort
        $DatabaseUrl = $uri.Uri.AbsoluteUri
    } elseif (-not $DatabaseUrl) {
        $DatabaseUrl = 'postgresql://koserver:koserver123@localhost:5432/ko_server'
    }
    $env:DATABASE_URL = $DatabaseUrl
    Write-Host 'Letter Admin hazırlanıyor...'
    & cargo build -p ko-letter-admin
    if ($LASTEXITCODE -ne 0) { throw 'ko-letter-admin derlenemedi.' }

    $backend = Join-Path $projectRoot 'target\debug\ko-letter-admin.exe'
    # Windows PowerShell 5 treats BOM-less UTF-8 .ps1 files as ANSI when they
    # are invoked directly. Reading explicitly as UTF-8 preserves Turkish UI.
    $uiPath = Join-Path $PSScriptRoot 'letter-admin.ps1'
    $uiSource = [System.IO.File]::ReadAllText($uiPath, $utf8)
    $uiScript = [ScriptBlock]::Create($uiSource)
    $targetLabel = if ($Target -eq 'Vps') { "VPS: $VpsHost" } else { 'LOCAL' }
    & $uiScript -Backend $backend -TargetLabel $targetLabel
}
finally {
    $env:DATABASE_URL = $previousDatabaseUrl
    if ($tunnel -and -not $tunnel.HasExited) { $tunnel.Kill(); $tunnel.WaitForExit() }
    Pop-Location
}
