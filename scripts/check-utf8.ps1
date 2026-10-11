# Strict UTF-8 validation for all git-tracked files.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-utf8.ps1
# Exit 0 = all tracked files are valid UTF-8; Exit 1 = at least one is corrupt.
$root = Split-Path $PSScriptRoot -Parent
$enc  = [System.Text.UTF8Encoding]::new($false, $true)
$bad  = @()
$files = & git -C $root -c core.quotepath=false ls-files
foreach ($f in $files) {
    $full = Join-Path $root $f
    if (-not (Test-Path -LiteralPath $full -ErrorAction SilentlyContinue)) { continue }
    try { [void]$enc.GetString([System.IO.File]::ReadAllBytes($full)) }
    catch { $bad += $f }
}
if ($bad.Count -eq 0) { Write-Output "OK   all tracked files are valid UTF-8"; exit 0 }
Write-Output "FAIL $($bad.Count) file(s) are NOT valid UTF-8:"
$bad | ForEach-Object { Write-Output "  BAD  $_" }
exit 1