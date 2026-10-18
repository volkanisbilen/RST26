$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$sqlFile = Join-Path $PSScriptRoot "sql\manes-survival-postgresql.sql"

if (-not (Test-Path $sqlFile)) {
    throw "SQL installer not found: $sqlFile"
}

$container = "korust-postgres"
$running = docker ps --filter "name=^/$container$" --format "{{.Names}}"
if ($running -ne $container) {
    Write-Host "Starting local PostgreSQL container..." -ForegroundColor Cyan
    docker compose -f (Join-Path $repo "docker-compose.local.yml") up -d postgres
}

$sqlFiles = @(
    $sqlFile,
    (Join-Path $repo "migrations\20260719000005_manes_survival_exact_coordinates.sql"),
    (Join-Path $repo "migrations\20260719000006_manes_survival_grade_rings.sql"),
    (Join-Path $repo "migrations\20260719000007_manes_survival_client_tables.sql")
)

Write-Host "Installing Manes Survival database records..." -ForegroundColor Cyan
foreach ($file in $sqlFiles) {
    if (-not (Test-Path $file)) {
        throw "Manes Survival SQL file not found: $file"
    }

    Write-Host ("  Applying " + (Split-Path $file -Leaf)) -ForegroundColor DarkCyan
    Get-Content -Raw -Encoding UTF8 $file |
        docker exec -i $container psql -v ON_ERROR_STOP=1 -U koserver -d ko_server

    if ($LASTEXITCODE -ne 0) {
        throw "Manes Survival database installation failed in $file (psql exit code $LASTEXITCODE)."
    }
}

Write-Host "Manes Survival zones, items, monsters, rings and client tables installed." -ForegroundColor Green
