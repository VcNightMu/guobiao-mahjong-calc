<#
  bench-plan.ps1 -- repeatable release-only timing benchmark for `gbmj <hand> --plan`.

  R8 recorded a one-off measurement (debug 2626 ms / release 166 ms for the worst case
  `1112345678999m`). A one-off number is not a benchmark: this script makes the number
  repeatable and self-checking. It does NOT change or optimise any algorithm.

  Implemented hard rules:
    1. release binary only; debug numbers are never used for conclusions;
    2. freshness gate: `target\release\gbmj.exe` mtime must be >= the newest `src\**\*.rs`
       mtime. A stale exe is a fake benchmark, so the run FAILS (use -Rebuild to rebuild
       with `cargo build --release --no-default-features` and then re-check);
    3. cases come from a UTF-8 TSV (`<id><TAB><args>`), so this file stays 7-bit ASCII and
       PowerShell 5.1 never has to decode CJK inside the script itself;
    4. >= 4 cases x -Runs runs (default 5): min / median / max plus the raw per-run array;
    5. correctness guard: the functional baseline `docs\BASELINE_*.txt` (9 cases) is
       replayed BEFORE and AFTER the timing (line-exact, LF-normalized, trailing blank
       lines ignored) so a "faster but wrong" binary cannot pass;
    6. `scripts\gate-data\plan-bench-baseline.json` is written only with -UpdateBaseline;
       a normal run reads it and prints median deltas.

  Usage:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1 -Runs 7
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1 -Filter g
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1 -Rebuild
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1 -UpdateBaseline
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\bench-plan.ps1 -Json C:\tmp\b.json

  Exit codes: 0 = OK (guard passed); 2 = freshness failure; 3 = guard failure;
              4 = usage / IO failure.
  This file is intentionally 7-bit ASCII so Windows PowerShell 5.1 reads it identically on
  any ANSI code page.
#>
param(
    [int]$Runs = 5,
    [string]$Filter = '',
    [string]$Exe = '',
    [string]$CasesFile = '',
    [string]$Json = '',
    [switch]$Rebuild,
    [switch]$UpdateBaseline
)

$ErrorActionPreference = 'Continue'
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false) } catch { }

function Die {
    param([int]$Code, [string]$Message)
    Write-Host ("ERROR: " + $Message) -ForegroundColor Red
    exit $Code
}

$RepoRoot  = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$ExePath   = if ($Exe -ne '') { $Exe } else { Join-Path $RepoRoot 'target\release\gbmj.exe' }
$CasesPath = if ($CasesFile -ne '') { $CasesFile } else { Join-Path $RepoRoot 'scripts\gate-data\plan-bench-cases.tsv' }
$BenchPath = if ($Json -ne '') { $Json } else { Join-Path $RepoRoot 'scripts\gate-data\plan-bench-baseline.json' }
$DocBase   = @(Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'docs') -Filter 'BASELINE_*.txt' -File | Sort-Object Name)
if ($DocBase.Count -lt 1) { Die 4 ("functional baseline file not found under docs\ (pattern BASELINE_*.txt)") }
$DocBasePath = $DocBase[0].FullName

if ($Runs -lt 3) { Die 4 ("-Runs must be >= 3 (got $Runs); use -Runs 5 or more for a stable median") }
if (-not (Test-Path -LiteralPath $CasesPath)) { Die 4 ("cases file not found: $CasesPath") }

# ---------------------------------------------------------------- 1. freshness gate
function Get-NewestSrc {
    return (Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'src') -Recurse -File -Filter '*.rs' |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1)
}
function Assert-Fresh {
    $exeItem = Get-Item -LiteralPath $ExePath
    $srcNewest = Get-NewestSrc
    Write-Host ("freshness: exe {0} mtime={1} | newest src {2} mtime={3}" -f `
        $exeItem.Name, $exeItem.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'), `
        $srcNewest.Name, $srcNewest.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))
    if ($exeItem.LastWriteTime -lt $srcNewest.LastWriteTime) { return $false }
    return $true
}

if (-not (Test-Path -LiteralPath $ExePath)) {
    if (-not $Rebuild) { Die 2 ("release binary not found: $ExePath -- build it with: cargo build --release --no-default-features") }
}
if ($Rebuild -or -not (Test-Path -LiteralPath $ExePath)) {
    Push-Location $RepoRoot
    try {
        Write-Host "rebuilding: cargo build --release --no-default-features"
        & cargo build --release --no-default-features 2>&1 | ForEach-Object { Write-Host ("  " + $_) }
        $buildExit = $LASTEXITCODE
    } finally { Pop-Location }
    if ($buildExit -ne 0) { Die 2 ("release build failed (exit $buildExit)") }
}
if (-not (Assert-Fresh)) {
    Die 2 "STALE release binary (exe older than newest src) -- rebuild with -Rebuild, then re-run"
}

# ---------------------------------------------------------------- 2. cases
$caseRows = @()
foreach ($line in [System.IO.File]::ReadAllLines($CasesPath, [System.Text.Encoding]::UTF8)) {
    $t = "$line".Trim()
    if ($t -eq '' -or $t.StartsWith('#')) { continue }
    $parts = $t -split "`t", 2
    if ($parts.Count -lt 2) { Die 4 ("bad TSV row (expected '<id><TAB><args>'): $t") }
    $caseRows += [pscustomobject]@{ Id = $parts[0].Trim(); Args = $parts[1].Trim() }
}
if ($Filter -ne '') { $caseRows = @($caseRows | Where-Object { $_.Id -like ("*" + $Filter + "*") -or $_.Args -like ("*" + $Filter + "*") }) }
if ($caseRows.Count -eq 0) { Die 4 ("no case matched -Filter '$Filter'") }
if ($caseRows.Count -lt 4) { Die 4 ("need at least 4 cases for a meaningful benchmark (got $($caseRows.Count))") }

# ---------------------------------------------------------------- 3. correctness guard
function Invoke-Case {
    param([string]$ArgsText)
    $argArr = @($ArgsText -split '\s+' | Where-Object { $_ -ne '' })
    return @(& $ExePath @argArr 2>&1)
}
function Normalize-Lines {
    param([string[]]$Lines)
    $conv = @()
    foreach ($l in $Lines) { $conv += ("$l" -replace "`r$", '') }
    $n = $conv.Count
    while ($n -gt 0 -and $conv[$n - 1] -eq '') { $n-- }
    if ($n -le 0) { return ,@() }
    return ,$conv[0..($n - 1)]
}
function Test-FunctionalBaseline {
    $raw = [System.IO.File]::ReadAllText($DocBasePath)
    $raw = $raw.TrimStart([char]0xFEFF)
    $textLines = [regex]::Split($raw, "`r`n|`n|`r")
    $cases = New-Object System.Collections.Generic.List[object]
    $cur = $null
    foreach ($line in $textLines) {
        if ($line -match '^=====\s*(\S+)\s*:\s*gbmj\s*(.*)$') {
            if ($null -ne $cur) { [void]$cases.Add($cur) }
            $cur = [pscustomobject]@{ Id = $Matches[1]; Args = $Matches[2].Trim(); Lines = (New-Object System.Collections.Generic.List[string]) }
        } elseif ($null -ne $cur) { [void]$cur.Lines.Add($line) }
    }
    if ($null -ne $cur) { [void]$cases.Add($cur) }
    $pass = 0
    foreach ($c in $cases) {
        $exp = Normalize-Lines $c.Lines.ToArray()
        $act = Normalize-Lines (Invoke-Case $c.Args)
        $same = ($exp.Count -eq $act.Count)
        if ($same) {
            for ($i = 0; $i -lt $exp.Count; $i++) { if ($exp[$i] -cne $act[$i]) { $same = $false; break } }
        }
        if ($same) { $pass++ } else { Write-Host ("  guard FAIL: {0} : gbmj {1}" -f $c.Id, $c.Args) -ForegroundColor Red }
    }
    return [pscustomobject]@{ Total = $cases.Count; Pass = $pass; Ok = ($cases.Count -gt 0 -and $pass -eq $cases.Count) }
}

Write-Host "guard: replaying functional baseline before timing"
$guardBefore = Test-FunctionalBaseline
Write-Host ("guard(before): {0}/{1} line-exact" -f $guardBefore.Pass, $guardBefore.Total)

# ---------------------------------------------------------------- 4. timing
function Get-Median {
    param([double[]]$Values)
    $s = @($Values | Sort-Object)
    $n = $s.Count
    if ($n -eq 0) { return 0.0 }
    if ($n % 2 -eq 1) { return [double]$s[[int](($n - 1) / 2)] }
    return ([double]$s[$n / 2 - 1] + [double]$s[$n / 2]) / 2.0
}

$results = @()
foreach ($c in $caseRows) {
    $argArr = @($c.Args -split '\s+' | Where-Object { $_ -ne '' })
    # the benchmark measures the `--plan` code path (R8); the correctness guard above must
    # never add `--plan`, so the flag is appended here and not inside Invoke-Case.
    $planArgs = @($argArr) + @('--plan')
    $warm = @(& $ExePath @planArgs 2>&1)   # warm-up (also yields the output-size proxy)
    $outLines = @($warm | Where-Object { "$_".Trim() -ne '' }).Count
    $raw = @()
    for ($i = 0; $i -lt $Runs; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & $ExePath @planArgs 2>&1 | Out-Null
        $sw.Stop()
        $raw += [Math]::Round($sw.Elapsed.TotalMilliseconds, 2)
    }
    $results += [pscustomobject]@{
        Id       = $c.Id
        Args     = $c.Args
        Runs     = $Runs
        Min      = ($raw | Measure-Object -Minimum).Minimum
        Median   = Get-Median $raw
        Max      = ($raw | Measure-Object -Maximum).Maximum
        OutLines = $outLines
        Raw      = $raw
    }
}

Write-Host "guard: replaying functional baseline after timing"
$guardAfter = Test-FunctionalBaseline
Write-Host ("guard(after): {0}/{1} line-exact" -f $guardAfter.Pass, $guardAfter.Total)

# ---------------------------------------------------------------- 5. report
Write-Host ""
Write-Host "=== gbmj --plan benchmark (release only) ==="
Write-Host ("exe: {0} | runs per case: {1} | cases: {2}" -f $ExePath, $Runs, $results.Count)
Write-Host ("case                          runs   min(ms)  median(ms)  max(ms)  outlines  raw")
foreach ($r in $results) {
    Write-Host ("{0,-28} {1,4} {2,8:N2} {3,10:N2} {4,8:N2} {5,8}  {6}" -f `
        $r.Id, $r.Runs, $r.Min, $r.Median, $r.Max, $r.OutLines, (($r.Raw | ForEach-Object { $_.ToString('0.##') }) -join ','))
}

# scale proxy note: the timing is a property of the hand shape; the printed output-line
# count is only a proxy and is NOT guaranteed to correlate monotonically with time.
Write-Host ""
Write-Host "note: 'outlines' is a coarse proxy for scale; the data below may be non-monotonic."

$bench = [pscustomobject]@{
    version   = 1
    capturedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    exe       = $ExePath.Substring($RepoRoot.Length + 1)
    exeMtimeUtc = (Get-Item -LiteralPath $ExePath).LastWriteTime.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    newestSrc = (Get-NewestSrc).Name
    newestSrcMtimeUtc = (Get-NewestSrc).LastWriteTime.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    runs      = $Runs
    guard     = [pscustomobject]@{ before = ("{0}/{1}" -f $guardBefore.Pass, $guardBefore.Total); after = ("{0}/{1}" -f $guardAfter.Pass, $guardAfter.Total) }
    cases     = @($results | ForEach-Object {
        [pscustomobject]@{ id = $_.Id; args = $_.Args; minMs = $_.Min; medianMs = $_.Median; maxMs = $_.Max; outLines = $_.OutLines; rawMs = $_.Raw }
    })
}
$jsonText = $bench | ConvertTo-Json -Depth 6

# The guard MUST be checked before any write. -UpdateBaseline is the only path that records
# numbers, so checking it at the very end (as this script first did) let a "faster but wrong"
# binary overwrite the recorded baseline and only then exit 3. Diagnostic reporting above still
# runs first, so a failing guard is still fully visible.
if (-not $guardBefore.Ok -or -not $guardAfter.Ok) { Die 3 "functional baseline guard failed -- refusing to record a benchmark" }

if ($UpdateBaseline) {
    $dir = Split-Path -Parent $BenchPath
    if ($dir -and -not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [System.IO.File]::WriteAllText($BenchPath, $jsonText, [System.Text.UTF8Encoding]::new($false))
    Write-Host ("baseline written: {0}" -f $BenchPath) -ForegroundColor Green
} else {
    Write-Host ("baseline not written (read-only run); file: {0}" -f $BenchPath)
    if (Test-Path -LiteralPath $BenchPath) {
        $old = [System.IO.File]::ReadAllText($BenchPath) | ConvertFrom-Json
        Write-Host "median deltas vs existing baseline:"
        foreach ($r in $results) {
            $o = @($old.cases | Where-Object { $_.id -eq $r.Id })
            if ($o.Count -gt 0 -and [double]$o[0].medianMs -gt 0) {
                $d = (([double]$r.Median - [double]$o[0].medianMs) / [double]$o[0].medianMs) * 100.0
                Write-Host ("  {0,-28} base={1,8:N2} now={2,8:N2} delta={3,7:N1}%" -f $r.Id, [double]$o[0].medianMs, [double]$r.Median, $d)
            } else {
                Write-Host ("  {0,-28} (no baseline entry)" -f $r.Id)
            }
        }
    } else {
        Write-Host ("baseline file does not exist yet: {0} (create it with -UpdateBaseline)" -f $BenchPath)
    }
}

# Guard already checked above the write block; reaching here means both replays were 9/9.
exit 0
