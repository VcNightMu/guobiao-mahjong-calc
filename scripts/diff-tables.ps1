#Requires -Version 5.1
<#
    diff-tables.ps1 -- differential verifier for the DIAGNOSTIC fan-table section only.

    Why this exists
      The CLI prints only the point-win fan table by default. The self-draw and last-tile
      tables are reachable solely through the diagnostic switch, so they were never covered
      by the default-output differential. This script appends that switch to every case and
      compares ONLY the part that follows the diagnostic banner.

    Banner (kept as Base64 because this file is pure ASCII)
      The decoded banner is the three-equals sign, a space, the four-character Chinese
      section title, a space, the switch name in parentheses, a space, three equals signs.

    Caliber and known limitation (recorded honestly)
      Classification is three-way:
        * BOTH sides print the banner  -> compare the banner section only.
        * NEITHER side prints it       -> compare the WHOLE output (stdout + stderr + exit code);
                                          this is the normal case for plan-scan arguments.
        * exactly ONE side prints it   -> EXPECTED-legacy-lacks-switch, because that build cannot
                                          emit the four tables at all (older build without the switch).
      A genuine old-versus-new probe differential therefore needs two builds that BOTH contain the
      switch. The recommended usage is a same-binary determinism run (its per-case digests double
      as a reference snapshot) plus a cross-binary run of two switch-bearing commits.

    Parameters (no hard-coded paths)
      -OldExe -NewExe -CasesFile -OutDir [-FailOnUnexpected]
      CasesFile uses the same tab-separated layout as diff-cli.ps1
      (case id, source, expectation, arguments joined by the pipe character).

    Temporary files
      Per-case captures live in a unique sub-directory of the system temp directory and are
      removed on exit (normal end and terminating errors). -OutDir receives final artefacts
      only, so a transient file can never be mistaken for repository residue.

    Exit codes
      0 = no unexpected differences and no verdict mismatch
      1 = at least one unexpected difference, or a verdict mismatch
      2 = bad parameters or unreadable inputs

    Owner: tester (L2). Task: T71.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OldExe,
    [Parameter(Mandatory = $true)][string]$NewExe,
    [Parameter(Mandatory = $true)][string]$CasesFile,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [switch]$FailOnUnexpected
)

$ErrorActionPreference = 'Stop'

$switchName = '--dump-tables'
$banner = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('PT09IOeVquihqOiviuaWre+8iC0tZHVtcC10YWJsZXPvvIkgPT09'))

function Fail-Param([string]$msg) {
    Write-Host ('[diff-tables] ERROR: ' + $msg) -ForegroundColor Red
    exit 2
}

foreach ($p in @($OldExe, $NewExe, $CasesFile)) {
    if (-not (Test-Path -LiteralPath $p)) { Fail-Param ('path not found: ' + $p) }
}
if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('diff-tables-' + $PID + '-' + ([System.Guid]::NewGuid().ToString('N').Substring(0, 8)))
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

trap {
    if ($script:tempRoot -and (Test-Path -LiteralPath $script:tempRoot)) {
        Remove-Item -LiteralPath $script:tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host ('[diff-tables] ERROR: ' + $_.Exception.Message) -ForegroundColor Red
    exit 2
}

function Get-Text([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return '' }
    return ([System.IO.File]::ReadAllText($path, [System.Text.UTF8Encoding]::new($false)) -replace "`r`n", "`n")
}

function Get-Section([string]$text) {
    if ($text -eq '') { return '' }
    $lines = $text -split "`n"
    $idx = -1
    for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -ceq $banner) { $idx = $i } }
    if ($idx -lt 0) { return '' }
    return (($lines[$idx..($lines.Count - 1)]) -join "`n")
}

function Get-Sha([string]$text) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { return [System.BitConverter]::ToString($sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($text))).Replace('-', '').Substring(0, 16) }
    finally { $sha.Dispose() }
}

function Invoke-One([string]$exe, [string[]]$argv, [string]$tag) {
    $outFile = Join-Path $tempRoot ('.tmp-' + $tag + '.out')
    $errFile = Join-Path $tempRoot ('.tmp-' + $tag + '.err')
    foreach ($f in @($outFile, $errFile)) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    $exit = -1
    $prev = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $exe @argv 1>$outFile 2>$errFile
        $exit = $LASTEXITCODE
    } catch {
        $exit = -999
    } finally {
        $ErrorActionPreference = $prev
    }
    $stdout = Get-Text $outFile
    $stderr = Get-Text $errFile
    foreach ($f in @($outFile, $errFile)) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    return [pscustomobject]@{
        exit      = $exit
        stdout    = $stdout
        stderr    = $stderr
        whole     = ($stdout + "`n" + $stderr)
        hasBanner = ($stdout -match [regex]::Escape($banner))
        section   = (Get-Section $stdout)
    }
}

$cases = New-Object System.Collections.ArrayList
$lineNo = 0
foreach ($line in [System.IO.File]::ReadAllLines($CasesFile, [System.Text.UTF8Encoding]::new($false))) {
    $lineNo++
    if ($line.Trim() -eq '' -or $line.TrimStart().StartsWith('#')) { continue }
    $parts = $line -split "`t"
    if ($parts.Count -lt 4) { Fail-Param ('case set line ' + $lineNo + ' has fewer than 4 tab-separated fields') }
    $argv = @()
    if ($parts[3].Trim() -ne '') { $argv = $parts[3].Split('|') }
    [void]$cases.Add([pscustomobject]@{ id = $parts[0]; src = $parts[1]; expect = $parts[2]; argv = ($argv + @($switchName)) })
}
if ($cases.Count -eq 0) { Fail-Param 'case set is empty' }

$oldInfo = Get-Item -LiteralPath $OldExe
$newInfo = Get-Item -LiteralPath $NewExe
$sameBinary = ($oldInfo.FullName -eq $newInfo.FullName)
$stamp = Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz'

Write-Host ('[diff-tables] cases=' + $cases.Count + ' mode=' + $(if ($sameBinary) { 'same-binary-determinism' } else { 'cross-binary' }) + ' old=' + $oldInfo.FullName + ' new=' + $newInfo.FullName) -ForegroundColor Cyan

$rows = New-Object System.Collections.ArrayList
$zero = 0; $zeroWhole = 0; $expectedLegacy = 0; $unexpected = 0; $mismatch = 0
$idx = 0
foreach ($c in $cases) {
    $idx++
    $a = Invoke-One $OldExe $c.argv ('o' + $idx)
    $b = Invoke-One $NewExe $c.argv ('n' + $idx)
    $verdict = 'same'
    $scope = 'section'
    if ($a.hasBanner -and $b.hasBanner) {
        $identical = (($a.section -ceq $b.section) -and ($a.exit -eq $b.exit))
        if (-not $identical) { $verdict = 'UNEXPECTED' }
    } elseif ($a.hasBanner -or $b.hasBanner) {
        # One side cannot emit the diagnostic banner at all (older build without the switch).
        $verdict = 'EXPECTED-legacy-lacks-switch'
        $scope = 'legacy'
    } else {
        # Neither side opens a diagnostic section (typical for plan-scan cases): fall back to a
        # whole-output comparison (stdout + stderr + exit code) instead of mislabelling legacy.
        $scope = 'whole'
        $identical = (($a.whole -ceq $b.whole) -and ($a.exit -eq $b.exit))
        if ($identical) { $verdict = 'same-whole' } else { $verdict = 'UNEXPECTED' }
    }
    switch ($verdict) {
        'same' { $zero++ }
        'same-whole' { $zeroWhole++ }
        'EXPECTED-legacy-lacks-switch' { $expectedLegacy++ }
        'UNEXPECTED' { $unexpected++ }
        default { $mismatch++ }
    }
    [void]$rows.Add([pscustomobject]@{
            id = $c.id; src = $c.src; verdict = $verdict; scope = $scope
            exitOld = $a.exit; exitNew = $b.exit
            bannerOld = $a.hasBanner; bannerNew = $b.hasBanner
            sectionShaOld = (Get-Sha $a.section); sectionShaNew = (Get-Sha $b.section)
            wholeShaOld = (Get-Sha $a.whole); wholeShaNew = (Get-Sha $b.whole)
        })
    if ($verdict -eq 'UNEXPECTED') {
        Write-Host ('  [' + $verdict + '] ' + $c.id + '  exit ' + $a.exit + '/' + $b.exit) -ForegroundColor Yellow
    }
}

$resultPath = Join-Path $OutDir 'diff-tables-results.tsv'
$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine(('id', 'source', 'verdict', 'scope', 'exitOld', 'exitNew', 'bannerOld', 'bannerNew', 'sectionShaOld', 'sectionShaNew', 'wholeShaOld', 'wholeShaNew') -join "`t")
foreach ($r in $rows) {
    [void]$sb.AppendLine(($r.id, $r.src, $r.verdict, $r.scope, $r.exitOld, $r.exitNew, $r.bannerOld, $r.bannerNew, $r.sectionShaOld, $r.sectionShaNew, $r.wholeShaOld, $r.wholeShaNew) -join "`t")
}
[System.IO.File]::WriteAllText($resultPath, $sb.ToString(), [System.Text.UTF8Encoding]::new($false))

$summaryPath = Join-Path $OutDir 'diff-tables-summary.txt'
$lines = New-Object System.Collections.ArrayList
[void]$lines.Add('diff-tables summary')
[void]$lines.Add('timestamp            : ' + $stamp)
[void]$lines.Add('mode                 : ' + $(if ($sameBinary) { 'same-binary-determinism' } else { 'cross-binary' }))
[void]$lines.Add('old binary           : ' + $oldInfo.FullName)
[void]$lines.Add('old binary sha256    : ' + (Get-FileHash -LiteralPath $OldExe -Algorithm SHA256).Hash)
[void]$lines.Add('new binary           : ' + $newInfo.FullName)
[void]$lines.Add('new binary sha256    : ' + (Get-FileHash -LiteralPath $NewExe -Algorithm SHA256).Hash)
[void]$lines.Add('case file            : ' + (Resolve-Path -LiteralPath $CasesFile).Path)
[void]$lines.Add('switch appended      : ' + $switchName)
[void]$lines.Add('cases total          : ' + $cases.Count)
[void]$lines.Add('zero-diff (section)  : ' + $zero)
[void]$lines.Add('zero-diff (whole)    : ' + $zeroWhole + '   (neither side opens a diagnostic section, e.g. plan-scan cases)')
[void]$lines.Add('expected-legacy      : ' + $expectedLegacy + '   (exactly one side has no banner: that build cannot emit the four tables at all)')
[void]$lines.Add('UNEXPECTED-diff      : ' + $unexpected)
[void]$lines.Add('verdict-mismatch     : ' + $mismatch)
[void]$lines.Add('compared scope       : section scope when BOTH sides print the banner; whole-output (stdout + stderr + exit) when NEITHER does; one-sided banner is reported as expected-legacy')
[void]$lines.Add('result table         : ' + $resultPath)
[System.IO.File]::WriteAllText($summaryPath, (($lines -join "`r`n") + "`r`n"), [System.Text.UTF8Encoding]::new($false))

$stray = @(Get-ChildItem -LiteralPath $OutDir -Force -Filter '.tmp-*' -ErrorAction SilentlyContinue).Count
Write-Host ('[diff-tables] zero-diff(section)=' + $zero + ' zero-diff(whole)=' + $zeroWhole + ' expected-legacy=' + $expectedLegacy + ' UNEXPECTED=' + $unexpected + ' mismatch=' + $mismatch + '  (stray temp files in -OutDir: ' + $stray + ')') -ForegroundColor Cyan
if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue }
Write-Host ('[diff-tables] results: ' + $resultPath) -ForegroundColor Cyan
Write-Host ('[diff-tables] summary: ' + $summaryPath) -ForegroundColor Cyan

if ($unexpected -gt 0 -or $mismatch -gt 0) { exit 1 }
exit 0
