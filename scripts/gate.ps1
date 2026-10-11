<#
    scripts\gate.ps1 - local one-shot gate for guobiao-mahjong-calc
    Windows PowerShell 5.1 compatible (no PS7-only syntax).

    Usage:
      powershell -NoProfile -ExecutionPolicy Bypass -File scripts\gate.ps1
                 [-UpdateBaseline] [-SkipFmt] [-StrictFmt] [-SkipClippy] [-StrictClippy]

    Blocking items (any FAIL => exit 1):
      1 build:        cargo build --no-default-features (debug), warning: lines must be 0
      2 test-list:    cargo test --no-default-features -- --list vs gate-data\tests-baseline.json
      3 test-result:  cargo test --no-default-features, failed == 0 and passed >= baseline count
      6 utf8:         scripts\check-utf8.ps1 exit 0
      7 functional:   docs baseline txt vs target\debug\gbmj.exe, 9/9 line-exact (LF)

    Advisory items (WARN by default, FAIL only with -Strict*):
      4 clippy:       cargo clippy ... -- -D warnings   (-SkipClippy / -StrictClippy)
      5 fmt:          cargo fmt --all -- --check        (-SkipFmt / -StrictFmt)
                      NEVER runs cargo fmt (never reformats sources).

    -UpdateBaseline: re-list tests, print diff, ask for interactive y confirmation, then write JSON
                     (also accepts 'y' piped on stdin; no confirmation => nothing written).

    This file is intentionally 7-bit ASCII so Windows PowerShell 5.1 reads it identically on any
    ANSI code page; non-ASCII output strings are carried as Base64-encoded UTF-8 literals.
#>
param(
    [switch]$UpdateBaseline,
    [switch]$SkipFmt,
    [switch]$StrictFmt,
    [switch]$SkipClippy,
    [switch]$StrictClippy
)

function ConvertFrom-B64Utf8 {
    param([string]$Value)
    return [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($Value))
}

$ErrorActionPreference = 'Continue'
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) } catch { }

# Non-ASCII strings as Base64(UTF-8): pending-decision notices + the docs baseline file name.
$MsgClippyPending = ConvertFrom-B64Utf8 '5b6F6KOB5a6a77ya6ZyA5YWI5L+u5aSNIHswfSDmnaEgbGludO+8iHNyYy/vvIzmnKzku7vliqHlhpnojIPlm7TlpJbvvIk='
$MsgFmtPending    = ConvertFrom-B64Utf8 '5b6F6KOB5a6a77ya6ZyA5YWI5L+u5aSN5qC85byP5beu5byC77yIezB9IOS4quaWh+S7tiAvIHsxfSDlpITniYfmrrXvvIzmnKzku7vliqHlhpnojIPlm7TlpJbvvIk='
$DocBaselineName  = ConvertFrom-B64Utf8 'ZG9jc1xCQVNFTElORV/ml6fkuozov5vliLbooYzkuLrln7rlh4YudHh0'

$RepoRoot     = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$GateDataDir  = Join-Path $RepoRoot 'scripts\gate-data'
$BaselinePath = Join-Path $GateDataDir 'tests-baseline.json'
$Utf8Script   = Join-Path $RepoRoot 'scripts\check-utf8.ps1'
$DocBaseline  = Join-Path $RepoRoot $DocBaselineName
$ExePath      = Join-Path $RepoRoot 'target\debug\gbmj.exe'
$TotalSw      = [System.Diagnostics.Stopwatch]::StartNew()
$Items        = New-Object System.Collections.Generic.List[object]
$FailNames    = New-Object System.Collections.Generic.List[string]

function Add-Result {
    param([string]$Name, [string]$Status, [double]$Seconds)
    [void]$Items.Add([pscustomobject]@{ Name = $Name; Status = $Status; Seconds = $Seconds })
    $color = 'Gray'
    if ($Status -eq 'PASS') { $color = 'Green' }
    elseif ($Status -eq 'WARN') { $color = 'Yellow' }
    elseif ($Status -eq 'FAIL') { $color = 'Red' }
    Write-Host ("[{0}] {1} ({2:N1}s)" -f $Status, $Name, $Seconds) -ForegroundColor $color
    if ($Status -eq 'FAIL') { [void]$FailNames.Add($Name) }
}

function Write-Detail {
    param([string]$Text, [string]$Color = 'Gray')
    Write-Host ("       {0}" -f $Text) -ForegroundColor $Color
}

function Get-CargoTestNames {
    Push-Location $RepoRoot
    try { $out = & cargo test --no-default-features -- --list 2>&1 }
    finally { Pop-Location }
    $list = New-Object System.Collections.Generic.List[string]
    foreach ($l in $out) {
        $t = "$l".TrimEnd()
        if ($t -match '^(.+): test$') { [void]$list.Add($Matches[1]) }
    }
    return $list.ToArray()
}

function Read-BaselineJson {
    if (-not (Test-Path -LiteralPath $BaselinePath)) { return $null }
    $txt = [System.IO.File]::ReadAllText($BaselinePath)
    return ($txt | ConvertFrom-Json)
}

function Write-BaselineJson {
    param([string[]]$Names)
    $sorted = @($Names | Sort-Object)
    $ts   = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    $head = ("$(& git -C $RepoRoot rev-parse --short HEAD)").Trim()
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($n in $sorted) { [void]$lines.Add('    "' + $n.Replace('\', '\\').Replace('"', '\"') + '"') }
    $json = "{`r`n  ""version"": 1,`r`n  ""capturedAtUtc"": ""$ts"",`r`n  ""capturedFromHead"": ""$head"",`r`n  ""count"": $($sorted.Count),`r`n  ""names"": [`r`n" + ($lines -join ",`r`n") + "`r`n  ]`r`n}`r`n"
    if (-not (Test-Path -LiteralPath $GateDataDir)) { New-Item -ItemType Directory -Force -Path $GateDataDir | Out-Null }
    [System.IO.File]::WriteAllText($BaselinePath, $json, [System.Text.UTF8Encoding]::new($false))
}

function Normalize-Lines {
    param([string[]]$Lines)
    $conv = @()
    if ($null -ne $Lines) { foreach ($l in $Lines) { $conv += ("$l" -replace "`r$", '') } }
    $n = $conv.Count
    while ($n -gt 0 -and $conv[$n - 1] -eq '') { $n-- }
    if ($n -le 0) { return ,@() }
    return ,$conv[0..($n - 1)]
}

Write-Host "=== gbmj local gate ===" -ForegroundColor Cyan
Write-Host ("repo : {0}" -f $RepoRoot)
Write-Host ("mode : UpdateBaseline={0} SkipFmt={1} StrictFmt={2} SkipClippy={3} StrictClippy={4}" -f [bool]$UpdateBaseline, [bool]$SkipFmt, [bool]$StrictFmt, [bool]$SkipClippy, [bool]$StrictClippy)
Write-Host ""

# ---------------------------------------------------------------- -UpdateBaseline
if ($UpdateBaseline) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Write-Host "[..] update baseline: cargo test --no-default-features -- --list"
    $newNames = @(Get-CargoTestNames)
    $oldNames = @()
    $oldBase = Read-BaselineJson
    if ($null -ne $oldBase) { $oldNames = @($oldBase.names) }
    $newSet = @{}; foreach ($n in $newNames) { $newSet[$n] = $true }
    $oldSet = @{}; foreach ($n in $oldNames) { $oldSet[$n] = $true }
    $added   = @($newNames | Where-Object { -not $oldSet.ContainsKey($_) })
    $removed = @($oldNames | Where-Object { -not $newSet.ContainsKey($_) })
    Write-Host ("  parsed={0} old={1} added={2} removed={3}" -f $newNames.Count, $oldNames.Count, $added.Count, $removed.Count)
    if ($removed.Count -gt 0) {
        Write-Host "  removed (would disappear from baseline):"
        $removed | ForEach-Object { Write-Host ("    - {0}" -f $_) }
    }
    if ($added.Count -gt 0) {
        Write-Host "  added (new tests):"
        $added | ForEach-Object { Write-Host ("    + {0}" -f $_) }
    }
    $answer = ''
    try { $answer = Read-Host "Write baseline to scripts\gate-data\tests-baseline.json? (type y to confirm)" } catch { $answer = '' }
    if ($null -eq $answer) { $answer = '' }
    $answer = "$answer".Trim()
    if ($answer -eq 'y' -or $answer -eq 'Y' -or $answer -eq 'yes' -or $answer -eq 'YES') {
        Write-BaselineJson $newNames
        Write-Host ("  baseline written: count={0} ({1:N1}s)" -f $newNames.Count, $sw.Elapsed.TotalSeconds) -ForegroundColor Green
        Write-Host "  re-run without -UpdateBaseline for the full gate."
        exit 0
    }
    Write-Host "  aborted: no 'y' confirmation received; baseline unchanged." -ForegroundColor Yellow
    exit 0
}

# ---------------------------------------------------------------- 1. build
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "[1/7] build: cargo build --no-default-features"
Push-Location $RepoRoot
try { $buildOut = & cargo build --no-default-features 2>&1; $buildExit = $LASTEXITCODE }
finally { Pop-Location }
$warnCount = 0
$warnSample = @()
foreach ($l in $buildOut) {
    if ("$l" -match '^warning:' -and "$l" -notmatch 'generated [0-9]+ warning') {
        $warnCount++
        if ($warnSample.Count -lt 5) { $warnSample += "$l" }
    }
}
if ($buildExit -eq 0 -and $warnCount -eq 0) {
    Write-Detail ("basis: exit code = {0}; 'warning:' lines = {1}" -f $buildExit, $warnCount)
    Add-Result 'build (blocking)' 'PASS' $sw.Elapsed.TotalSeconds
} else {
    Write-Detail ("basis: exit code = {0}; 'warning:' lines = {1}" -f $buildExit, $warnCount) 'Red'
    foreach ($w in $warnSample) { Write-Detail ("  {0}" -f $w) 'Red' }
    Add-Result 'build (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
}

# ---------------------------------------------------------------- 2. test list lock
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "[2/7] test-list lock: cargo test --no-default-features -- --list"
$curNames = @(Get-CargoTestNames)
$Baseline = Read-BaselineJson
if ($null -eq $Baseline) {
    Write-Detail ("basis: baseline JSON not found: {0}" -f $BaselinePath) 'Red'
    Write-Detail "hint: run with -UpdateBaseline and confirm with 'y' to generate it, then re-run." 'Red'
    Add-Result 'test-list lock (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
} else {
    $baseNames = @($Baseline.names)
    $curSet = @{}; foreach ($n in $curNames) { $curSet[$n] = $true }
    $baseSet = @{}; foreach ($n in $baseNames) { $baseSet[$n] = $true }
    $missing = @($baseNames | Where-Object { -not $curSet.ContainsKey($_) })
    $added   = @($curNames | Where-Object { -not $baseSet.ContainsKey($_) })
    Write-Detail ("current names = {0}; baseline names = {1}; baseline count field = {2}; added = {3}" -f $curNames.Count, $baseNames.Count, $Baseline.count, $added.Count)
    if ($added.Count -gt 0) { Write-Detail ("added (allowed): {0}" -f ($added -join ', ')) }
    if ($missing.Count -gt 0) {
        Write-Detail ("basis: {0} baseline name(s) missing/renamed => FAIL" -f $missing.Count) 'Red'
        foreach ($m in $missing) { Write-Detail ("  missing: {0}" -f $m) 'Red' }
        Add-Result 'test-list lock (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
    } else {
        Write-Detail "basis: every baseline name still present (a rename shows up as missing => FAIL); additions allowed"
        Add-Result 'test-list lock (blocking)' 'PASS' $sw.Elapsed.TotalSeconds
    }
}

# ---------------------------------------------------------------- 3. test result
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "[3/7] test result: cargo test --no-default-features"
Push-Location $RepoRoot
try { $testOut = & cargo test --no-default-features 2>&1; $testExit = $LASTEXITCODE }
finally { Pop-Location }
$passed = 0
$failed = 0
foreach ($l in $testOut) {
    $t = "$l".TrimEnd()
    if ($t -match 'test result:.*?([0-9]+) passed; ([0-9]+) failed') {
        $passed += [int]$Matches[1]
        $failed += [int]$Matches[2]
    }
}
$required = 0
if ($null -ne $Baseline -and $null -ne $Baseline.count) { $required = [int]$Baseline.count }
if ($failed -eq 0 -and $required -gt 0 -and $passed -ge $required) {
    Write-Detail ("basis: failed total = {0} (need 0); passed total = {1} (need >= {2}); exit = {3}" -f $failed, $passed, $required, $testExit)
    Add-Result 'test result (blocking)' 'PASS' $sw.Elapsed.TotalSeconds
} else {
    Write-Detail ("basis: failed total = {0} (need 0); passed total = {1} (need >= {2}); exit = {3}" -f $failed, $passed, $required, $testExit) 'Red'
    Add-Result 'test result (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
}

# ---------------------------------------------------------------- 4. clippy (advisory)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
if ($SkipClippy) {
    Write-Host "[4/7] clippy: skipped (-SkipClippy)"
    Write-Detail "basis: skipped by -SkipClippy"
    Add-Result 'clippy (advisory)' 'WARN' $sw.Elapsed.TotalSeconds
} else {
    Write-Host "[4/7] clippy: cargo clippy --no-default-features --all-targets -- -D warnings"
    Push-Location $RepoRoot
    try { $clipOut = & cargo clippy --no-default-features --all-targets -- -D warnings 2>&1; $clipExit = $LASTEXITCODE }
    finally { Pop-Location }
    # rustc/cargo's own diagnostic count (authoritative number of lints).
    $diagCount = $null
    foreach ($l in $clipOut) { if ("$l" -match 'could not compile .*due to ([0-9]+) previous error') { $diagCount = [int]$Matches[1] } }
    if ($null -eq $diagCount) { foreach ($l in $clipOut) { if ("$l" -match 'aborting due to ([0-9]+) previous error') { $diagCount = [int]$Matches[1] } } }
    if ($null -eq $diagCount) {
        $diagCount = @($clipOut | Where-Object { "$_" -match '^error(\[[^\]]+\])?: ' -and "$_" -notmatch 'could not compile|aborting due to|previous error|generated [0-9]+ warning' }).Count
    }
    # span-line counting: reproduces the L2-measured baseline (37 lines, fans 12).
    $spanCounts = @{}
    foreach ($l in $clipOut) {
        if ("$l" -match '^\s*-->\s+([^:]+):') {
            $f = $Matches[1].Trim().Replace('\', '/')
            if ($spanCounts.ContainsKey($f)) { $spanCounts[$f] = $spanCounts[$f] + 1 } else { $spanCounts[$f] = 1 }
        }
    }
    $lintCount = 0
    foreach ($v in $spanCounts.Values) { $lintCount += $v }
    $top5 = @($spanCounts.GetEnumerator() | Sort-Object -Property Value -Descending | Select-Object -First 5 | ForEach-Object { "{0}:{1}" -f $_.Key, $_.Value }) -join ', '
    Write-Detail ("exit = {0}; lint span-lines = {1}; rustc diagnostics = {2}; by-file top5 = [{3}]" -f $clipExit, $lintCount, $diagCount, $top5)
    if ($clipExit -eq 0 -and $lintCount -eq 0) {
        Add-Result 'clippy (advisory)' 'PASS' $sw.Elapsed.TotalSeconds
    } elseif ($StrictClippy) {
        Write-Detail ($MsgClippyPending -f $lintCount) 'Red'
        Write-Detail "basis: -StrictClippy => lint findings are blocking" 'Red'
        Add-Result 'clippy (advisory, -StrictClippy)' 'FAIL' $sw.Elapsed.TotalSeconds
    } else {
        Write-Detail "basis: advisory only (default WARN); pass -StrictClippy to make it blocking"
        Write-Detail ($MsgClippyPending -f $lintCount) 'Yellow'
        Add-Result 'clippy (advisory)' 'WARN' $sw.Elapsed.TotalSeconds
    }
}

# ---------------------------------------------------------------- 5. fmt (advisory)
$sw = [System.Diagnostics.Stopwatch]::StartNew()
if ($SkipFmt) {
    Write-Host "[5/7] fmt: skipped (-SkipFmt)"
    Write-Detail "basis: skipped by -SkipFmt (cargo fmt is never executed by this gate)"
    Add-Result 'fmt (advisory)' 'WARN' $sw.Elapsed.TotalSeconds
} else {
    Write-Host "[5/7] fmt check: cargo fmt --all -- --check (never runs cargo fmt)"
    Push-Location $RepoRoot
    try { $fmtOut = & cargo fmt --all -- --check 2>&1; $fmtExit = $LASTEXITCODE }
    finally { Pop-Location }
    $fmtLineCount = @($fmtOut).Count
    $fragCount = 0
    $fmtFiles = @{}
    foreach ($l in $fmtOut) {
        $t = "$l".TrimEnd()
        if ($t -match '^Diff in (.+)$') {
            $fragCount++
            $f = $Matches[1].TrimEnd(':').Trim()
            $f = $f -replace ' at line [0-9]+$', ''
            $f = ($f -replace ':[0-9]+$', '').Trim().Replace('\', '/')
            if ($fmtFiles.ContainsKey($f)) { $fmtFiles[$f] = $fmtFiles[$f] + 1 } else { $fmtFiles[$f] = 1 }
        }
    }
    $fmtFileCount = $fmtFiles.Count
    Write-Detail ("exit = {0}; 'Diff in' fragments = {1}; output lines = {2}; distinct files = {3}" -f $fmtExit, $fragCount, $fmtLineCount, $fmtFileCount)
    if ($fmtExit -eq 0 -and $fragCount -eq 0) {
        Add-Result 'fmt (advisory)' 'PASS' $sw.Elapsed.TotalSeconds
    } elseif ($StrictFmt) {
        Write-Detail ($MsgFmtPending -f $fmtFileCount, $fragCount) 'Red'
        Write-Detail "basis: -StrictFmt => formatting drift is blocking; sources were NOT modified" 'Red'
        Add-Result 'fmt (advisory, -StrictFmt)' 'FAIL' $sw.Elapsed.TotalSeconds
    } else {
        Write-Detail "basis: advisory only (default WARN); pass -StrictFmt to make it blocking; sources were NOT modified"
        Write-Detail ($MsgFmtPending -f $fmtFileCount, $fragCount) 'Yellow'
        Add-Result 'fmt (advisory)' 'WARN' $sw.Elapsed.TotalSeconds
    }
}

# ---------------------------------------------------------------- 6. utf8 gate
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host "[6/7] utf8 gate: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-utf8.ps1"
if (-not (Test-Path -LiteralPath $Utf8Script)) {
    Write-Detail ("basis: script not found: {0}" -f $Utf8Script) 'Red'
    Add-Result 'utf8 gate (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
} else {
    $utf8Out = & powershell -NoProfile -ExecutionPolicy Bypass -File $Utf8Script 2>&1
    $utf8Exit = $LASTEXITCODE
    Write-Detail ("basis: exit code = {0}; output = {1}" -f $utf8Exit, (@($utf8Out) -join ' | '))
    if ($utf8Exit -eq 0) { Add-Result 'utf8 gate (blocking)' 'PASS' $sw.Elapsed.TotalSeconds }
    else { Add-Result 'utf8 gate (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds }
}

# ---------------------------------------------------------------- 7. functional baseline 9/9
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Host ("[7/7] functional baseline: {0} vs target\debug\gbmj.exe" -f $DocBaselineName)
if (-not (Test-Path -LiteralPath $DocBaseline)) {
    Write-Detail ("basis: docs baseline not found: {0}" -f $DocBaseline) 'Red'
    Add-Result 'functional baseline (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
} elseif (-not (Test-Path -LiteralPath $ExePath)) {
    Write-Detail ("basis: exe not found: {0}" -f $ExePath) 'Red'
    Add-Result 'functional baseline (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds
} else {
    $raw = [System.IO.File]::ReadAllText($DocBaseline)
    $raw = $raw.TrimStart([char]0xFEFF)
    $textLines = [regex]::Split($raw, "\r\n|\n|\r")
    $cases = New-Object System.Collections.Generic.List[object]
    $cur = $null
    foreach ($line in $textLines) {
        if ($line -match '^=====\s*(\S+)\s*:\s*gbmj\s*(.*)$') {
            if ($null -ne $cur) { [void]$cases.Add($cur) }
            $cur = [pscustomobject]@{ Id = $Matches[1]; Args = $Matches[2].Trim(); Lines = (New-Object System.Collections.Generic.List[string]) }
        } elseif ($null -ne $cur) {
            [void]$cur.Lines.Add($line)
        }
    }
    if ($null -ne $cur) { [void]$cases.Add($cur) }
    $caseCount = 0
    $passCount = 0
    foreach ($c in $cases) {
        $caseCount++
        $exp = Normalize-Lines $c.Lines.ToArray()
        if ($c.Args -eq '') { $actual = @(& $ExePath 2>&1) }
        else { $argArr = @($c.Args -split '\s+'); $actual = @(& $ExePath @argArr 2>&1) }
        $act = Normalize-Lines $actual
        $same = ($exp.Count -eq $act.Count)
        $diffIdx = -1
        if ($same) {
            for ($i = 0; $i -lt $exp.Count; $i++) {
                if ($exp[$i] -cne $act[$i]) { $same = $false; $diffIdx = $i; break }
            }
        } else {
            $diffIdx = [Math]::Min($exp.Count, $act.Count)
        }
        if ($same) {
            $passCount++
            Write-Host ("       [PASS] {0} : gbmj {1}" -f $c.Id, $c.Args) -ForegroundColor Green
        } else {
            Write-Host ("       [FAIL] {0} : gbmj {1} (expected {2} lines, actual {3} lines, first diff at line {4})" -f $c.Id, $c.Args, $exp.Count, $act.Count, ($diffIdx + 1)) -ForegroundColor Red
            $e = '(missing)'; $a = '(missing)'
            if ($diffIdx -lt $exp.Count) { $e = $exp[$diffIdx] }
            if ($diffIdx -lt $act.Count) { $a = $act[$diffIdx] }
            Write-Host ("         expected: {0}" -f $e) -ForegroundColor Red
            Write-Host ("         actual  : {0}" -f $a) -ForegroundColor Red
        }
    }
    Write-Detail ("basis: cases = {0}; pass = {1}; fail = {2} (requirement: 9/9 line-exact, LF-normalized, trailing blank lines ignored)" -f $caseCount, $passCount, ($caseCount - $passCount))
    if ($caseCount -eq 9 -and $passCount -eq 9) { Add-Result 'functional baseline (blocking)' 'PASS' $sw.Elapsed.TotalSeconds }
    else { Add-Result 'functional baseline (blocking)' 'FAIL' $sw.Elapsed.TotalSeconds }
}

# ---------------------------------------------------------------- summary
Write-Host ""
Write-Host "=== summary ===" -ForegroundColor Cyan
foreach ($it in $Items) { Write-Host ("  {0,-6} {1,-34} {2,6:N1}s" -f $it.Status, $it.Name, $it.Seconds) }
$p = @($Items | Where-Object { $_.Status -eq 'PASS' }).Count
$w = @($Items | Where-Object { $_.Status -eq 'WARN' }).Count
$f = @($Items | Where-Object { $_.Status -eq 'FAIL' }).Count
Write-Host ("  PASS {0} / WARN {1} / FAIL {2}" -f $p, $w, $f)
Write-Host ("  total elapsed: {0:N1}s" -f $TotalSw.Elapsed.TotalSeconds)
if ($f -gt 0) {
    Write-Host ("  FAILED items: {0}" -f ($FailNames -join ', ')) -ForegroundColor Red
    exit 1
}
Write-Host "  gate OK (exit 0)" -ForegroundColor Green
exit 0
