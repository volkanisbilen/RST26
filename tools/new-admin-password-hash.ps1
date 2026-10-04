$ErrorActionPreference = 'Stop'

Write-Host 'Admin parolasini girin (yazarken ekranda gorunmez):'
$builder = [System.Text.StringBuilder]::new()
while ($true) {
    $key = [Console]::ReadKey($true)
    if ($key.Key -eq [ConsoleKey]::Enter) { break }
    if ($key.Key -eq [ConsoleKey]::Backspace) {
        if ($builder.Length -gt 0) { $builder.Length-- }
        continue
    }
    if (-not [char]::IsControl($key.KeyChar)) { [void]$builder.Append($key.KeyChar) }
}

$password = $builder.ToString()
$builder.Clear() | Out-Null
if ($password.Length -lt 12) { throw 'Guvenlik icin parola en az 12 karakter olmali.' }

$salt = [byte[]]::new(16)
[System.Security.Cryptography.RandomNumberGenerator]::Fill($salt)
$passwordBytes = [System.Text.Encoding]::UTF8.GetBytes($password)
$derived = [System.Security.Cryptography.Rfc2898DeriveBytes]::Pbkdf2(
    $passwordBytes,
    $salt,
    310000,
    [System.Security.Cryptography.HashAlgorithmName]::SHA256,
    32
)
[Array]::Clear($passwordBytes, 0, $passwordBytes.Length)
$password = $null

$saltHex = [Convert]::ToHexString($salt).ToLowerInvariant()
$hashHex = [Convert]::ToHexString($derived).ToLowerInvariant()
[Array]::Clear($derived, 0, $derived.Length)
Write-Output "pbkdf2_sha256`$310000`$$saltHex`$$hashHex"
