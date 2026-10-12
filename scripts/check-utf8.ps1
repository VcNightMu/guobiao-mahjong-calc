# Strict UTF-8 validation for all git-tracked TEXT files.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-utf8.ps1
# Exit 0 = all tracked TEXT files are valid UTF-8; Exit 1 = at least one is corrupt.
#
# WHY TEXT ONLY (revised 2026-10-12, T18 -- read before "fixing" this back):
#   This script exists to stop source/docs from being corrupted by a UTF-8 <-> GBK
#   round-trip (a past incident). Validating BINARY assets as UTF-8 is a category
#   error: every PNG is by definition "not valid UTF-8", so tracking image evidence
#   (docs/evidence/*.png) would pin the gate red forever.
#   => TEXT EXTENSION ALLOWLIST: files whose extension is in $textExt are validated
#      byte-for-byte; every other extension is SKIPPED BUT COUNTED AND REPORTED, so a
#      reviewer can see exactly what was skipped and how much (nothing is released
#      silently).
#   => Files WITHOUT an extension (LICENSE, Makefile, ...) are treated as TEXT.
#   => .txt stays in scope: the evidence truth files are text (a UTF-8 BOM is valid
#      UTF-8 and passes; BOM is never a reason to skip).
#   KNOWN LIMITATION: an extension outside $textExt (e.g. a future .foo) is NOT
#      validated. When a new text type appears, ADD its extension to $textExt here;
#      do not rewrite this into "validate everything except images".
#   NOTE: keep this file ASCII-only -- scripts\gate.ps1 invokes it with Windows
#      PowerShell 5.1, which decodes BOM-less .ps1 files as ANSI (same reason
#      gate.ps1 inlines its Chinese messages as Base64).

$root = Split-Path $PSScriptRoot -Parent
$enc  = [System.Text.UTF8Encoding]::new($false, $true)
# .tsv added 2026-10-12 (T34/T36): the diff and bench case tables are TEXT holding CJK tile
# notation; leaving them out would have skipped them silently as "non-text".
$textExt = @('.rs', '.md', '.toml', '.txt', '.tsv', '.ps1', '.js', '.json', '.yml', '.yaml', '.html', '.css', '.lock')
$bad = @()
$skipped = @{}
$checked = 0
$files = & git -C $root -c core.quotepath=false ls-files
foreach ($f in $files) {
    $full = Join-Path $root $f
    if (-not (Test-Path -LiteralPath $full -ErrorAction SilentlyContinue)) { continue }
    $leaf = Split-Path $f -Leaf
    $ext  = [System.IO.Path]::GetExtension($leaf).ToLowerInvariant()
    # A dotfile like .gitignore/.gitattributes has no REAL extension (Path.GetExtension
    # returns ".gitignore" for it) -- treat it as TEXT, never as an unknown-binary skip.
    if ($leaf.StartsWith('.') -and $leaf.IndexOf('.', 1) -lt 0) { $ext = '' }
    if ($ext -ne '' -and ($textExt -notcontains $ext)) {
        if ($skipped.ContainsKey($ext)) { $skipped[$ext]++ } else { $skipped[$ext] = 1 }
        continue
    }
    $checked++
    try { [void]$enc.GetString([System.IO.File]::ReadAllBytes($full)) }
    catch { $bad += $f }
}
$skipTotal = 0
$skipDesc  = 'none'
if ($skipped.Count -gt 0) {
    $skipTotal = ($skipped.Values | Measure-Object -Sum).Sum
    $skipDesc  = (($skipped.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Key) x $($_.Value)" }) -join ', ')
}
if ($bad.Count -eq 0) {
    Write-Output "OK   all $checked tracked TEXT files are valid UTF-8 (skipped $skipTotal non-text: $skipDesc)"
    exit 0
}
Write-Output "FAIL $($bad.Count) file(s) are NOT valid UTF-8:"
$bad | ForEach-Object { Write-Output "  BAD  $_" }
exit 1
