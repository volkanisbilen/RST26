param(
    [Parameter(Mandatory = $true)]
    [string]$ClientExe,

    [string]$StoreUrl = 'http://84.247.183.23:8081/pus/'
)

$sourcePath = (Resolve-Path -LiteralPath $ClientExe).Path
$originalUrl = 'https://knightmall.nttgame.com/authentication.mgame'
$originalBytes = [Text.Encoding]::ASCII.GetBytes($originalUrl)
$replacementBytes = [Text.Encoding]::ASCII.GetBytes($StoreUrl)

if ($replacementBytes.Length -gt $originalBytes.Length) {
    throw "StoreUrl en fazla $($originalBytes.Length) ASCII karakter olabilir."
}

$clientBytes = [IO.File]::ReadAllBytes($sourcePath)
$matches = [Collections.Generic.List[int]]::new()
for ($offset = 0; $offset -le $clientBytes.Length - $originalBytes.Length; $offset++) {
    $found = $true
    for ($index = 0; $index -lt $originalBytes.Length; $index++) {
        if ($clientBytes[$offset + $index] -ne $originalBytes[$index]) {
            $found = $false
            break
        }
    }
    if ($found) {
        $matches.Add($offset)
        $offset += $originalBytes.Length - 1
    }
}

if ($matches.Count -ne 1) {
    throw "Beklenen 2625 PUS adresi tam olarak bir kez bulunmalıydı; bulunan: $($matches.Count). Dosya değiştirilmedi."
}

$backupPath = "$sourcePath.pus-backup"
if (-not (Test-Path -LiteralPath $backupPath)) {
    Copy-Item -LiteralPath $sourcePath -Destination $backupPath
}

$patchOffset = $matches[0]
for ($index = 0; $index -lt $originalBytes.Length; $index++) {
    $clientBytes[$patchOffset + $index] = 0
}
[Array]::Copy($replacementBytes, 0, $clientBytes, $patchOffset, $replacementBytes.Length)
[IO.File]::WriteAllBytes($sourcePath, $clientBytes)

$hash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash
Write-Host "PUS WebView2 adresi güncellendi: $StoreUrl" -ForegroundColor Green
Write-Host ("Offset: 0x{0:X}" -f $patchOffset)
Write-Host "Yedek: $backupPath"
Write-Host "SHA256: $hash"
