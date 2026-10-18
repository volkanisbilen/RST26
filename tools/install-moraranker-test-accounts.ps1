$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$sqlFile = Join-Path $repo "migrations\20260804000002_seed_moraranker_test_accounts.sql"
$container = "korust-postgres"

if (-not (Test-Path $sqlFile)) {
    throw "MORANKER account SQL file not found: $sqlFile"
}

$running = docker ps --filter "name=^/$container$" --format "{{.Names}}"
if ($running -ne $container) {
    Write-Host "Starting local PostgreSQL container..." -ForegroundColor Cyan
    docker compose -f (Join-Path $repo "docker-compose.local.yml") up -d postgres
}

Write-Host "Creating MORANKER test accounts a1-a10..." -ForegroundColor Cyan
Get-Content -Raw -Encoding UTF8 $sqlFile |
    docker exec -i $container psql -v ON_ERROR_STOP=1 -U koserver -d ko_server

if ($LASTEXITCODE -ne 0) {
    throw "MORANKER test account installation failed (psql exit code $LASTEXITCODE)."
}

$accountList = docker exec $container psql -U koserver -d ko_server -At -c @"
SELECT str_account_id
FROM tb_user
WHERE str_account_id IN ('a1','a2','a3','a4','a5','a6','a7','a8','a9','a10')
ORDER BY length(str_account_id), str_account_id;
"@

if ($LASTEXITCODE -ne 0) {
    throw "MORANKER account verification query failed."
}

$accounts = @($accountList | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if ($accounts.Count -ne 10) {
    throw "Expected 10 MORANKER accounts, found $($accounts.Count)."
}

Write-Host "MORANKER test accounts are ready:" -ForegroundColor Green
$accounts | ForEach-Object { Write-Host "  $_" -ForegroundColor DarkCyan }
Write-Host "All accounts use the same stored password value as account a." -ForegroundColor Green
Write-Host "Nation and characters are intentionally left empty for in-client creation." -ForegroundColor Green
