#Requires -Version 5.1
<#
    diff-cli.ps1 -- reproducible CLI differential verifier (old binary vs new binary).

    Purpose
      Run a parameterised case set against two builds of `gbmj` and compare, per case:
        * stdout bytes (raw)
        * stderr bytes (after noise stripping)
        * exit code
      Reported as zero-diff / expected-diff / unexpected-diff. Any unexpected difference
      (or an "expected-diff" case that turned out identical) is a FAIL.

    Parameters (no hard-coded one-off paths)
      -OldExe     path to the baseline binary
      -NewExe     path to the candidate binary
      -CasesFile  TSV case set:  id<TAB>source<TAB>expectation<TAB>arg1|arg2|...
                  expectation = "same"   -> outputs MUST be identical
                                "diff:R" -> a difference is EXPECTED (R = recorded reason)
                  blank lines and lines starting with '#' are ignored
      -OutDir     directory for FINAL evidence outputs only (created if missing).
                  Per-case temporary files (.tmp-*.out/.err) live in a UNIQUE sub-directory
                  of the system temp dir ([System.IO.Path]::GetTempPath()), named
                  "diff-cli-<pid>-<random>", and that directory is removed on exit
                  (normal end and terminating errors). -OutDir therefore never shows
                  transient .tmp-* files, so `git status` cannot mistake them for residue.
      -TimeoutSec per-case wall clock budget (informational; default 60)
      -FailOnUnexpected  exit 1 when any "same" case differs (default ON)

    Noise-stripping rule (why it exists)
      T32 hit a FALSE difference: when a native command's stderr is captured into a
      variable/string (2>&1), PowerShell wraps each line in an ErrorRecord and appends
      context lines that contain the *script's own line number*
      ("[zh] zai suo wei zhi hang:N" / "At line:N" / "+ CategoryInfo ..." /
      "+ FullyQualifiedErrorId ..."). Two fixes are applied together:
        (1) stdout/stderr are redirected to FILES, so no ErrorRecord formatting happens, and
        (2) any remaining line matching the noise patterns below is stripped before compare.
      Stripped line patterns (regex, applied to stdout and stderr):
        ^\s*\+?\s*<ZH_SUOZAIWEIZHI>          (the localised "at position/line" banner)
        ^\s*<ZH_SHIYONG>.+?<ZH_DIAOYONG>      (the localised "invoking method ... exception")
        ^\s*At line[: ]
        ^\s*\+\s*CategoryInfo
        ^\s*\+\s*FullyQualifiedErrorId
        NativeCommandError
        ^\s*At .*\.ps1:\d+ char:\d+\s*$      (T65/T57: English-locale caller banner)
        ^\s*\+\s*&                            (T65/T57: echoed source line of the native call)
        ^\s*\+\s*~+\s*$                       (T65/T57: caret underline)
      Real stderr text (e.g. "cuo wu: an pai zhang shu bu fu ...") never matches these
      patterns and is therefore preserved.
      Both binaries are launched identically, so any residue affects both sides equally;
      the rule is applied anyway so that the comparison is script-independent.

    ASCII policy (and why the Chinese patterns are Base64)
      This script is PURE ASCII. Windows PowerShell 5.1 reads a BOM-less file as ANSI, so a
      CJK byte can be 0x5C (backslash) and would escape the closing quote of a string
      literal, breaking the parser. The two localised patterns are therefore stored as
      Base64 and decoded with ConvertFrom-B64Utf8 (same technique as scripts/gate.ps1).
      Decoded values:
        5omA5Zyo5L2N572u -> "suo zai wei zhi" banner (localised 'at position')
        5L2/55So          -> "shi yong"   (localised 'using/invoking')
        6LCD55So          -> "diao yong"  (localised 'invoking')
      Case data (the TSV) may contain UTF-8 Chinese tile names; it is read as UTF-8.

    Exit codes
      0 = no unexpected differences
      1 = at least one unexpected difference, or an expected-diff case that was identical
      2 = bad parameters / unreadable inputs

    Owner: tester (L2). Task: T34 / task-30; temp-dir migration + coverage expansion: T48 / task-41.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OldExe,
    [Parameter(Mandatory = $true)][string]$NewExe,
    [Parameter(Mandatory = $true)][string]$CasesFile,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [int]$TimeoutSec = 60,
    [switch]$FailOnUnexpected
)

$ErrorActionPreference = 'Stop'

function ConvertFrom-B64Utf8 {
    param([string]$Value)
    return [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($Value))
}

function Fail-Param([string]$msg) {
    Write-Host ("[diff-cli] ERROR: " + $msg) -ForegroundColor Red
    exit 2
}

foreach ($p in @($OldExe, $NewExe, $CasesFile)) {
    if (-not (Test-Path -LiteralPath $p)) { Fail-Param ("path not found: " + $p) }
}
if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }

# Per-case temp files go to a unique directory under the system temp path, never to -OutDir.
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('diff-cli-' + $PID + '-' + ([System.Guid]::NewGuid().ToString('N').Substring(0, 8)))
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

# Terminating errors (e.g. an unhandled cmdlet failure) must still clean the temp directory.
trap {
    if ($script:tempRoot -and (Test-Path -LiteralPath $script:tempRoot)) {
        Remove-Item -LiteralPath $script:tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host ("[diff-cli] ERROR: " + $_.Exception.Message) -ForegroundColor Red
    exit 2
}

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
    # T65/T57 additions (minimal): Windows PowerShell 5.1 caller context that is NOT
    # locale-localised and NOT stable across script revisions:
    #   "At C:\...\script.ps1:6 char:1"  (English-locale caller banner; the line number
    #                                     changes whenever the script's own source moves)
    #   "+ & $exe @argv 1>$outFile 2>$errFile"  (echoed source line of the native call)
    #   "+ ~~~~~~~~~~~~~~~~~~~~~~~~"            (caret underline; length follows the source line)
    '^\s*At .*\.ps1:\d+ char:\d+\s*$',
    '^\s*\+\s*&',
    '^\s*\+\s*~+\s*$'
)

function Get-StableText([string]$path) {
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

function Get-Sha([string]$text) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { return [System.BitConverter]::ToString($sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($text))).Replace('-', '').Substring(0, 16) }
    finally { $sha.Dispose() }
}

function Invoke-Case([string]$exe, [string[]]$argv, [string]$tag) {
    $outFile = Join-Path $tempRoot (".tmp-" + $tag + ".out")
    $errFile = Join-Path $tempRoot (".tmp-" + $tag + ".err")
    foreach ($f in @($outFile, $errFile)) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    $exit = -1
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # NOTE (Windows PowerShell 5.1): a native command that writes to stderr raises a
    # NativeCommandError, which becomes TERMINATING while $ErrorActionPreference='Stop'
    # (observed as exit -999 on both sides). EAP is therefore relaxed for the call; the
    # exit code is taken from $LASTEXITCODE and stderr is captured raw by the file redirect.
    $prevEap = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $exe @argv 1>$outFile 2>$errFile
        $exit = $LASTEXITCODE
    } catch {
        $exit = -999
    } finally {
        $ErrorActionPreference = $prevEap
        $sw.Stop()
    }
    $stdout = Get-StableText $outFile
    $stderr = Get-StableText $errFile
    foreach ($f in @($outFile, $errFile)) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    return [pscustomobject]@{
        exit   = $exit
        stdout = $stdout
        stderr = $stderr
        ms     = [int]$sw.Elapsed.TotalMilliseconds
    }
}

$cases = New-Object System.Collections.ArrayList
$lineNo = 0
foreach ($line in [System.IO.File]::ReadAllLines($CasesFile, [System.Text.UTF8Encoding]::new($false))) {
    $lineNo++
    if ($line.Trim() -eq '' -or $line.TrimStart().StartsWith('#')) { continue }
    $parts = $line -split "`t"
    if ($parts.Count -lt 4) { Fail-Param ("case set line " + $lineNo + " has fewer than 4 tab-separated fields") }
    $argv = @()
    if ($parts[3].Trim() -ne '') { $argv = $parts[3].Split('|') }
    [void]$cases.Add([pscustomobject]@{ id = $parts[0]; src = $parts[1]; expect = $parts[2]; argv = $argv })
}
if ($cases.Count -eq 0) { Fail-Param 'case set is empty' }

$oldInfo = Get-Item -LiteralPath $OldExe
$newInfo = Get-Item -LiteralPath $NewExe
$stamp = Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz'

Write-Host ("[diff-cli] cases=" + $cases.Count + " old=" + $oldInfo.FullName + " new=" + $newInfo.FullName) -ForegroundColor Cyan

$rows = New-Object System.Collections.ArrayList
$zero = 0; $expectedDiff = 0; $unexpected = 0; $mismatch = 0
$idx = 0
foreach ($c in $cases) {
    $idx++
    $a = Invoke-Case $OldExe $c.argv ("o" + $idx)
    $b = Invoke-Case $NewExe $c.argv ("n" + $idx)
    $identical = (($a.stdout -ceq $b.stdout) -and ($a.stderr -ceq $b.stderr) -and ($a.exit -eq $b.exit))
    $isExpected = $c.expect -ne 'same'
    if ($identical) { $verdict = $(if ($isExpected) { 'FAIL-expected-but-identical' } else { 'same' }) }
    else { $verdict = $(if ($isExpected) { 'expected-diff' } else { 'UNEXPECTED' }) }
    switch ($verdict) {
        'same' { $zero++ }
        'expected-diff' { $expectedDiff++ }
        'UNEXPECTED' { $unexpected++ }
        default { $mismatch++ }
    }
    [void]$rows.Add([pscustomobject]@{
            id = $c.id; src = $c.src; expect = $c.expect; verdict = $verdict
            exitOld = $a.exit; exitNew = $b.exit
            stdoutShaOld = (Get-Sha $a.stdout); stdoutShaNew = (Get-Sha $b.stdout)
            stderrShaOld = (Get-Sha $a.stderr); stderrShaNew = (Get-Sha $b.stderr)
            msOld = $a.ms; msNew = $b.ms
        })
    if ($verdict -ne 'same') {
        Write-Host ("  [" + $verdict + "] " + $c.id + "  exit " + $a.exit + "/" + $b.exit) -ForegroundColor Yellow
    }
}

$resultPath = Join-Path $OutDir 'diff-cli-results.tsv'
$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine(('id', 'source', 'expectation', 'verdict', 'exitOld', 'exitNew', 'stdoutShaOld', 'stdoutShaNew', 'stderrShaOld', 'stderrShaNew', 'msOld', 'msNew') -join "`t")
foreach ($r in $rows) {
    [void]$sb.AppendLine(($r.id, $r.src, $r.expect, $r.verdict, $r.exitOld, $r.exitNew, $r.stdoutShaOld, $r.stdoutShaNew, $r.stderrShaOld, $r.stderrShaNew, $r.msOld, $r.msNew) -join "`t")
}
[System.IO.File]::WriteAllText($resultPath, $sb.ToString(), [System.Text.UTF8Encoding]::new($false))

$summaryPath = Join-Path $OutDir 'diff-cli-summary.txt'
$slow = $rows | Sort-Object { [Math]::Max($_.msOld, $_.msNew) } -Descending | Select-Object -First 5
$lines = New-Object System.Collections.ArrayList
[void]$lines.Add('diff-cli summary')
[void]$lines.Add('timestamp            : ' + $stamp)
[void]$lines.Add('old binary           : ' + $oldInfo.FullName)
[void]$lines.Add('old binary mtime     : ' + $oldInfo.LastWriteTime.ToString('yyyy-MM-ddTHH:mm:ss'))
[void]$lines.Add('old binary sha256    : ' + (Get-FileHash -LiteralPath $OldExe -Algorithm SHA256).Hash)
[void]$lines.Add('new binary           : ' + $newInfo.FullName)
[void]$lines.Add('new binary mtime     : ' + $newInfo.LastWriteTime.ToString('yyyy-MM-ddTHH:mm:ss'))
[void]$lines.Add('new binary sha256    : ' + (Get-FileHash -LiteralPath $NewExe -Algorithm SHA256).Hash)
[void]$lines.Add('case file            : ' + (Resolve-Path -LiteralPath $CasesFile).Path)
[void]$lines.Add('cases total          : ' + $cases.Count)
[void]$lines.Add('zero-diff            : ' + $zero)
[void]$lines.Add('expected-diff        : ' + $expectedDiff)
[void]$lines.Add('UNEXPECTED-diff      : ' + $unexpected)
[void]$lines.Add('verdict-mismatch     : ' + $mismatch + '   (expected-diff case that was identical)')
[void]$lines.Add('')
[void]$lines.Add('top-5 slowest cases (max of old/new ms):')
foreach ($s in $slow) { [void]$lines.Add('  ' + $s.id + ' old=' + $s.msOld + 'ms new=' + $s.msNew + 'ms') }
[void]$lines.Add('')
[void]$lines.Add('noise-strip rule     : stdout/stderr redirected to files; lines matching the localised at-position banner, the localised invoking banner, "At line", "+ CategoryInfo", "+ FullyQualifiedErrorId" or "NativeCommandError" are removed (Base64-encoded zh patterns inside the script)')
[void]$lines.Add('result table         : ' + $resultPath)
[System.IO.File]::WriteAllText($summaryPath, (($lines -join "`r`n") + "`r`n"), [System.Text.UTF8Encoding]::new($false))

Write-Host ("[diff-cli] zero-diff=" + $zero + " expected-diff=" + $expectedDiff + " UNEXPECTED=" + $unexpected + " verdict-mismatch=" + $mismatch) -ForegroundColor Cyan
Write-Host ("[diff-cli] results: " + $resultPath) -ForegroundColor Cyan
Write-Host ("[diff-cli] summary: " + $summaryPath) -ForegroundColor Cyan

$strayTmp = @(Get-ChildItem -LiteralPath $OutDir -Force -Filter '.tmp-*' -ErrorAction SilentlyContinue)
Write-Host ("[diff-cli] temp dir used: " + $tempRoot + "  (stray .tmp-* in -OutDir: " + $strayTmp.Count + ")") -ForegroundColor Cyan
if (Test-Path -LiteralPath $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue }
Write-Host ("[diff-cli] temp dir removed: " + (-not (Test-Path -LiteralPath $tempRoot)))

if ($unexpected -gt 0 -or $mismatch -gt 0) { exit 1 }
exit 0
