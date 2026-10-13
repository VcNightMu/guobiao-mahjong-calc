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

    Noise stripping (same rule as diff-cli.ps1)
      Per-case stdout/stderr are redirected to files in a unique temp directory, so PowerShell
      never formats a native stderr stream into an ErrorRecord at capture time; the wrapper lines
      that PS still writes into the redirected stderr file are stripped before hashing. Without
      that second line of defence wholeSha/sectionSha would embed the CALL SITE line number of
      this script, so adding a comment would silently change the stored fingerprints.

    Caliber (expectation column is the source of truth)
      Scope selection is unchanged (three-way):
        * BOTH sides print the banner  -> compare the banner section only.
        * NEITHER side prints it       -> compare the WHOLE output (stdout + stderr + exit code);
                                          this is the normal case for plan-scan arguments.
        * exactly ONE side prints it   -> EXPECTED-legacy-lacks-switch, because that build cannot
                                          emit the four tables at all (older build without the switch).
      Within the section scope AND the whole scope the verdict now also reads the cases
      expectation column (same as diff-cli.ps1):
        * expectation "same" and outputs differ -> UNEXPECTED (never relaxed; still exit 1).
        * expectation "diff:<reason>" and outputs differ -> expected-section-diff /
          expected-whole-diff, and the reason string is carried into the result table and the
          per-reason grouping in the summary.
      A genuine old-versus-new probe differential therefore needs two builds that BOTH contain the
      switch. The recommended usage is a same-binary determinism run (its per-case digests double
      as a reference snapshot) plus a cross-binary run of two switch-bearing commits.

    Known limitation (accepted by the Lead on 2026-10-13; keep this documented)
      A case whose expectation is "diff:<reason>" is reported as same / same-whole when the current
      binary pair no longer differs (both sides already carry the fix). That is deliberate: the
      diff: reasons are registered against a specific PRE-FIX build, and the pair changes between
      tasks, so forcing a mismatch would turn this tool red for every other pair and make it
      unusable. Cost: a fix that is later reverted is NOT detected here (it shows up only as same).
      A case whose expectation is "same" is NEVER relaxed: any difference is still UNEXPECTED.

    Reason-column caliber
      The reason string carried into the result table echoes the cases expectation column, i.e. the
      reason registered against the PRE-FIX build. It is not guaranteed to describe the actual
      difference for the binary pair in use (an older build that already contains the switch can
      still differ for an unrelated fan-family reason).

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

function ConvertFrom-B64Utf8 {
    param([string]$Value)
    return [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($Value))
}

# Same noise-strip rule as diff-cli.ps1 (T65/T57). Pure ASCII: the three localised patterns are
# Base64(UTF-8) so that no CJK byte can ever break the parser. Decoded values:
#   5omA5Zyo5L2N572u -> localised "at position/line" banner
#   5L2/55So          -> localised "using/invoking"
#   6LCD55So          -> localised "invoking"
$zhPos = ConvertFrom-B64Utf8 '5omA5Zyo5L2N572u'
$zhUse = ConvertFrom-B64Utf8 '5L2/55So'
$zhInv = ConvertFrom-B64Utf8 '6LCD55So'
$noisePatterns = @(
    ('^\s*\+?\s*' + $zhPos),
    ('^\s*' + $zhUse + '.+?' + $zhInv),
    '^\s*At line:\d+ char:\d+\s*$',
    '^\s*\+\s*CategoryInfo',
    '^\s*\+\s*FullyQualifiedErrorId',
    'NativeCommandError',
    '^\s*At .*\.ps1:\d+ char:\d+\s*$',
    '^\s*\+\s*&',
    '^\s*\+\s*~+\s*$'
)

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
    # Same caliber as diff-cli.ps1: stdout/stderr are already redirected to files (no PS
    # ErrorRecord formatting on capture), AND the wrapper lines that PowerShell still writes into
    # the redirected stderr file are stripped. Without this second line of defence the digests
    # would embed the CALL SITE line number of THIS script, so adding a comment would change
    # wholeSha/sectionSha (evidence/fingerprint pollution, observed as 124 changed rows).
    if (-not (Test-Path -LiteralPath $path)) { return '' }
    $raw = [System.IO.File]::ReadAllText($path, [System.Text.UTF8Encoding]::new($false))
    $raw = $raw -replace "`r`n", "`n"
    $kept = New-Object System.Collections.Generic.List[string]
    foreach ($line in ($raw -split "`n")) {
        $isNoise = $false
        foreach ($np in $noisePatterns) { if ($line -match $np) { $isNoise = $true; break } }
        if (-not $isNoise) { [void]$kept.Add($line) }
    }
    return ($kept -join "`n")
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
$zero = 0; $zeroWhole = 0; $expectedLegacy = 0; $expectedSection = 0; $expectedWhole = 0
$unexpected = 0; $mismatch = 0
$reasons = @{}
$idx = 0
foreach ($c in $cases) {
    $idx++
    $a = Invoke-One $OldExe $c.argv ('o' + $idx)
    $b = Invoke-One $NewExe $c.argv ('n' + $idx)
    $verdict = 'same'
    $scope = 'section'
    $reason = ''
    $isExpected = ($c.expect -ne 'same')
    if ($a.hasBanner -and $b.hasBanner) {
        $identical = (($a.section -ceq $b.section) -and ($a.exit -eq $b.exit))
        if (-not $identical) {
            if ($isExpected) { $verdict = 'expected-section-diff'; $reason = $c.expect }
            else { $verdict = 'UNEXPECTED' }
        }
    } elseif ($a.hasBanner -or $b.hasBanner) {
        # One side cannot emit the diagnostic banner at all (older build without the switch).
        $verdict = 'EXPECTED-legacy-lacks-switch'
        $scope = 'legacy'
    } else {
        # Neither side opens a diagnostic section (typical for plan-scan cases): fall back to a
        # whole-output comparison (stdout + stderr + exit code) instead of mislabelling legacy.
        $scope = 'whole'
        $identical = (($a.whole -ceq $b.whole) -and ($a.exit -eq $b.exit))
        if ($identical) { $verdict = 'same-whole' }
        elseif ($isExpected) { $verdict = 'expected-whole-diff'; $reason = $c.expect }
        else { $verdict = 'UNEXPECTED' }
    }
    switch ($verdict) {
        'same' { $zero++ }
        'same-whole' { $zeroWhole++ }
        'EXPECTED-legacy-lacks-switch' { $expectedLegacy++ }
        'expected-section-diff' { $expectedSection++ }
        'expected-whole-diff' { $expectedWhole++ }
        'UNEXPECTED' { $unexpected++ }
        default { $mismatch++ }
    }
    if ($reason -ne '') {
        if ($reasons.ContainsKey($reason)) { $reasons[$reason] = $reasons[$reason] + 1 }
        else { $reasons[$reason] = 1 }
    }
    [void]$rows.Add([pscustomobject]@{
            id = $c.id; src = $c.src; expect = $c.expect; verdict = $verdict; scope = $scope; reason = $reason
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
[void]$sb.AppendLine(('id', 'source', 'expectation', 'verdict', 'scope', 'reason', 'exitOld', 'exitNew', 'bannerOld', 'bannerNew', 'sectionShaOld', 'sectionShaNew', 'wholeShaOld', 'wholeShaNew') -join "`t")
foreach ($r in $rows) {
    [void]$sb.AppendLine(($r.id, $r.src, $r.expect, $r.verdict, $r.scope, $r.reason, $r.exitOld, $r.exitNew, $r.bannerOld, $r.bannerNew, $r.sectionShaOld, $r.sectionShaNew, $r.wholeShaOld, $r.wholeShaNew) -join "`t")
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
[void]$lines.Add('expected-section-diff: ' + $expectedSection + '   (both sides print the banner, section differs, case expectation is diff:<reason>)')
[void]$lines.Add('expected-whole-diff  : ' + $expectedWhole + '   (neither side prints the banner, whole output differs, case expectation is diff:<reason>)')
[void]$lines.Add('expected-total       : ' + ($expectedSection + $expectedWhole) + '   (section ' + $expectedSection + ' + whole ' + $expectedWhole + ')')
[void]$lines.Add('expected-legacy      : ' + $expectedLegacy + '   (exactly one side has no banner: that build cannot emit the four tables at all)')
[void]$lines.Add('UNEXPECTED-diff      : ' + $unexpected + '   (outputs differ while the case expectation is same: never relaxed)')
[void]$lines.Add('verdict-mismatch     : ' + $mismatch)
[void]$lines.Add('expected by reason:')
foreach ($k in ($reasons.Keys | Sort-Object)) { [void]$lines.Add('  ' + $k + ' = ' + $reasons[$k]) }
[void]$lines.Add('compared scope       : section scope when BOTH sides print the banner; whole-output (stdout + stderr + exit) when NEITHER does; one-sided banner is reported as expected-legacy')
[void]$lines.Add('expectation column   : source of truth for diff: cases in BOTH the section and the whole scope; same cases stay strict (any difference is UNEXPECTED)')
[void]$lines.Add('known limitation     : a diff:<reason> case that does NOT differ under THIS binary pair is reported as same/same-whole and raises no mismatch (reasons are registered against a pre-fix build and the pair changes between tasks); a same case is never relaxed, but a later-reverted fix is not detected here')
[void]$lines.Add('reason column caliber: reason echoes the cases expectation column (registered against a pre-fix build) and is not guaranteed to describe the actual difference for the binary pair in use')
[void]$lines.Add('result table         : ' + $resultPath)
[System.IO.File]::WriteAllText($summaryPath, (($lines -join "`r`n") + "`r`n"), [System.Text.UTF8Encoding]::new($false))

$stray = @(Get-ChildItem -LiteralPath $OutDir -Force -Filter '.tmp-*' -ErrorAction SilentlyContinue).Count
Write-Host ('[diff-tables] zero-diff(section)=' + $zero + ' zero-diff(whole)=' + $zeroWhole + ' expected-section-diff=' + $expectedSection + ' expected-whole-diff=' + $expectedWhole + ' expected-total=' + ($expectedSection + $expectedWhole) + ' expected-legacy=' + $expectedLegacy + ' UNEXPECTED=' + $unexpected + ' mismatch=' + $mismatch + '  (stray temp files in -OutDir: ' + $stray + ')') -ForegroundColor Cyan
if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue }
Write-Host ('[diff-tables] results: ' + $resultPath) -ForegroundColor Cyan
Write-Host ('[diff-tables] summary: ' + $summaryPath) -ForegroundColor Cyan

if ($unexpected -gt 0 -or $mismatch -gt 0) { exit 1 }
exit 0
