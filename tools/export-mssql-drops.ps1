$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$migration = Join-Path $repo "migrations\20260817000004_sync_mssql_drop_tables.sql"
$tempDir = Join-Path $repo ".tmp-mssql-drops"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

$monsterFile = Join-Path $tempDir "monster_item.rows"
$groupFile = Join-Path $tempDir "make_item_group.rows"
$server = "DESKTOP-M2HE50H\JSTKO"
$database = "KO_DATABASE_SERVER_001"

function Invoke-DropExport {
    param(
        [Parameter(Mandatory = $true)][string]$Query,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)][string]$Label
    )

    for ($attempt = 1; $attempt -le 6; $attempt++) {
        $args = @("-S", $server, "-d", $database, "-E", "-w", "65535", "-y", "0", "-Q", $Query, "-o", $OutputPath)
        if (($attempt % 2) -eq 0) {
            $args += "-C"
        }
        & sqlcmd @args
        if ($LASTEXITCODE -eq 0) {
            return
        }
        Start-Sleep -Seconds 1
    }
    throw "$Label export failed after 6 attempts."
}

$monsterParts = @("CONVERT(varchar(12), COALESCE(sIndex, 0))")
for ($i = 1; $i -le 12; $i++) {
    $suffix = $i.ToString("00")
    $monsterParts += "CONVERT(varchar(12), COALESCE(iItem$suffix, 0))"
    $monsterParts += "CONVERT(varchar(12), COALESCE(sPersent$suffix, 0))"
}
$monsterExpr = ($monsterParts -join " + ',' + ")
$monsterQuery = "SET NOCOUNT ON; SELECT DISTINCT '(' + $monsterExpr + ')' FROM K_MONSTER_ITEM2369 ORDER BY 1;"

$groupParts = @()
for ($i = 1; $i -le 200; $i++) {
    $groupParts += "CONVERT(varchar(12), COALESCE(iItem_$i, 0))"
}
$groupExpr = ($groupParts -join " + ',' + ")
$groupQuery = "SET NOCOUNT ON; SELECT '(' + CONVERT(varchar(12), iItemGroupNum) + ',ARRAY[' + $groupExpr + ']::INTEGER[])' FROM MAKE_ITEM_GROUP ORDER BY iItemGroupNum;"

Invoke-DropExport -Query $monsterQuery -OutputPath $monsterFile -Label "K_MONSTER_ITEM2369"
Invoke-DropExport -Query $groupQuery -OutputPath $groupFile -Label "MAKE_ITEM_GROUP"

$monsterRows = @(Get-Content -LiteralPath $monsterFile | Where-Object { $_ -match '^\(' })
$groupRows = @(Get-Content -LiteralPath $groupFile | Where-Object { $_ -match '^\(' })
if ($monsterRows.Count -ne 2180) { throw "Expected 2180 distinct monster rows, got $($monsterRows.Count)." }
if ($groupRows.Count -ne 374) { throw "Expected 374 item-group rows, got $($groupRows.Count)." }

$columns = @("s_index")
for ($i = 1; $i -le 12; $i++) {
    $suffix = $i.ToString("00")
    $columns += "item$suffix"
    $columns += "percent$suffix"
}

$output = [System.Collections.Generic.List[string]]::new()
$output.Add("-- Exact snapshot imported from KO_DATABASE_SERVER_001 on DESKTOP-M2HE50H\JSTKO.")
$output.Add("-- Source: K_MONSTER_ITEM2369 (4349 physical rows, 2180 exact distinct rows).")
$output.Add("-- Source: MAKE_ITEM_GROUP (374 rows).")
$output.Add("DELETE FROM monster_item;")
$output.Add("INSERT INTO monster_item ($($columns -join ', ')) VALUES")
for ($i = 0; $i -lt $monsterRows.Count; $i++) {
    $suffix = if ($i -eq $monsterRows.Count - 1) { ";" } else { "," }
    $output.Add($monsterRows[$i] + $suffix)
}
$output.Add("")
$output.Add("DELETE FROM make_item_group;")
$output.Add("INSERT INTO make_item_group (group_num, items) VALUES")
for ($i = 0; $i -lt $groupRows.Count; $i++) {
    $suffix = if ($i -eq $groupRows.Count - 1) { ";" } else { "," }
    $output.Add($groupRows[$i] + $suffix)
}
$output.Add("")
$output.Add("DO `$`$")
$output.Add("BEGIN")
$output.Add("    IF (SELECT COUNT(*) FROM monster_item) <> 2180 THEN")
$output.Add("        RAISE EXCEPTION 'monster_item snapshot count mismatch';")
$output.Add("    END IF;")
$output.Add("    IF (SELECT COUNT(*) FROM make_item_group) <> 374 THEN")
$output.Add("        RAISE EXCEPTION 'make_item_group snapshot count mismatch';")
$output.Add("    END IF;")
$output.Add("END `$`$;")

[System.IO.File]::WriteAllLines($migration, $output, [System.Text.UTF8Encoding]::new($false))
Write-Host "Created $migration with $($monsterRows.Count) monster rows and $($groupRows.Count) groups."
