param(
    [Parameter(Mandatory = $false)]
    [string]$ClientExe = "C:\Users\volka\Downloads\KnightOnline2625\KnightOnline.exe",

    [Parameter(Mandatory = $false)]
    [string]$Reference2615Exe = "D:\ai\ko26xx\docs\reverse-engineering\2615\KnightOnLine_unpacked.exe"
)

$ErrorActionPreference = "Stop"

function Read-VaBytes {
    param(
        [string]$Path,
        [uint32]$VirtualAddress,
        [int]$Count
    )

    $imageBase = [uint32]0x00400000
    if ($VirtualAddress -lt $imageBase) {
        throw "Virtual address must be at or above 0x00400000."
    }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $stream.Position = [int64]($VirtualAddress - $imageBase)
        $bytes = New-Object byte[] $Count
        $read = $stream.Read($bytes, 0, $Count)
        if ($read -ne $Count) {
            throw "Could not read $Count bytes from VA 0x$($VirtualAddress.ToString('X8'))."
        }
        return $bytes
    }
    finally {
        $stream.Dispose()
    }
}

function Test-ByteArrayEqual {
    param(
        [byte[]]$Left,
        [byte[]]$Right
    )

    if ($Left.Length -ne $Right.Length) {
        return $false
    }
    for ($index = 0; $index -lt $Left.Length; $index++) {
        if ($Left[$index] -ne $Right[$index]) {
            return $false
        }
    }
    return $true
}

$expectedHash = "9C2A5309F5DC118D938D5CDB7753E272D4D5169659A71F0DB49043B6E5A285E4"
$actualHash = (Get-FileHash -LiteralPath $ClientExe -Algorithm SHA256).Hash
if ($actualHash -ne $expectedHash) {
    throw "Unexpected v2625 executable hash: $actualHash"
}

$versionFunction = Read-VaBytes -Path $ClientExe -VirtualAddress 0x007AE420 -Count 6
$expectedVersionFunction = [byte[]](0xB8, 0x41, 0x0A, 0x00, 0x00, 0xC3)
if (-not (Test-ByteArrayEqual -Left $versionFunction -Right $expectedVersionFunction)) {
    throw "The v2625 required-version function does not match MOV EAX,2625; RET."
}

$oldTable = Read-VaBytes -Path $Reference2615Exe -VirtualAddress 0x007AF568 -Count 253
$newTable = Read-VaBytes -Path $ClientExe -VirtualAddress 0x007B5258 -Count 253
if (-not (Test-ByteArrayEqual -Left $oldTable -Right $newTable)) {
    throw "Main GameServer opcode lookup table differs from the verified 2615 table."
}

Write-Host "Verified KnightOnline v2625 executable: $actualHash"
Write-Host "Required GameServer version: 2625"
Write-Host "Main 253-byte opcode lookup table: identical to verified v2615"
