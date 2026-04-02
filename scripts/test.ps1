#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Full test suite with detailed per-crate reporting for claude-cli-rs.
.PARAMETER Quick
    Only run cargo check + clippy (skip tests).
.PARAMETER ShowAll
    Show individual test results, not just summaries.
.PARAMETER StopOnFail
    Stop at the first failing stage.
.EXAMPLE
    .\scripts\test.ps1
    .\scripts\test.ps1 -Quick
    .\scripts\test.ps1 -ShowAll
    .\scripts\test.ps1 -StopOnFail
#>
[CmdletBinding()]
param(
    [switch]$Quick,
    [switch]$ShowAll,
    [switch]$StopOnFail
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# ── Helpers ──────────────────────────────────────────────────

function Write-StageHeader($num, $total, $name) {
    Write-Host ""
    Write-Host "[$num/$total] $name" -ForegroundColor Yellow
}

function Write-Pass($msg) { Write-Host "  [PASS] $msg" -ForegroundColor Green }
function Write-Fail($msg) { Write-Host "  [FAIL] $msg" -ForegroundColor Red }
function Write-Warn($msg) { Write-Host "  [WARN] $msg" -ForegroundColor DarkYellow }
function Write-Info($msg) { Write-Host "  $msg" }

# ── Main ─────────────────────────────────────────────────────

Write-Host "=== Claude CLI (Rust) - Test Suite ===" -ForegroundColor Cyan

$totalStages = if ($Quick) { 2 } else { 4 }
$stageResults = @()
$overallPass = $true

Push-Location $ProjectRoot
try {

    # ── Stage 1: cargo check ──

    Write-StageHeader 1 $totalStages "cargo check --workspace"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $checkOutput = cargo check --workspace 2>&1 | Out-String
    $checkExit = $LASTEXITCODE
    $sw.Stop()
    $elapsed = [math]::Round($sw.Elapsed.TotalSeconds, 1)

    $errorCount = ([regex]::Matches($checkOutput, '(?m)^error(\[E\d+\])?:')).Count
    $warnCount = ([regex]::Matches($checkOutput, '(?m)^warning:')).Count

    if ($checkExit -eq 0 -and $errorCount -eq 0) {
        Write-Pass "Passed (${elapsed}s) - $errorCount errors, $warnCount warnings"
        $stageResults += @{ Name = "check"; Pass = $true; Time = $elapsed }
    } else {
        Write-Fail "Failed (${elapsed}s) - $errorCount errors, $warnCount warnings"
        if ($ShowAll) { Write-Host $checkOutput }
        $stageResults += @{ Name = "check"; Pass = $false; Time = $elapsed }
        $overallPass = $false
        if ($StopOnFail) { throw "Stopped at cargo check" }
    }

    # ── Stage 2: cargo clippy ──

    Write-StageHeader 2 $totalStages "cargo clippy --workspace -- -D warnings"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $clippyOutput = cargo clippy --workspace -- -D warnings 2>&1 | Out-String
    $clippyExit = $LASTEXITCODE
    $sw.Stop()
    $elapsed = [math]::Round($sw.Elapsed.TotalSeconds, 1)

    $clippyWarns = ([regex]::Matches($clippyOutput, '(?m)^warning:')).Count

    if ($clippyExit -eq 0) {
        Write-Pass "Passed (${elapsed}s) - 0 warnings"
        $stageResults += @{ Name = "clippy"; Pass = $true; Time = $elapsed }
    } else {
        Write-Fail "Failed (${elapsed}s) - $clippyWarns warnings"
        if ($ShowAll) { Write-Host $clippyOutput }
        $stageResults += @{ Name = "clippy"; Pass = $false; Time = $elapsed }
        $overallPass = $false
        if ($StopOnFail) { throw "Stopped at cargo clippy" }
    }

    if ($Quick) {
        Write-Host ""
        Write-Host "Quick mode: skipping test stages." -ForegroundColor DarkGray
    } else {

        # ── Stage 3: cargo test ──

        Write-StageHeader 3 $totalStages "cargo test --workspace --exclude claude-cli-rs"
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $testOutput = cargo test --workspace --exclude claude-cli-rs 2>&1 | Out-String
        $testExit = $LASTEXITCODE
        $sw.Stop()
        $elapsed = [math]::Round($sw.Elapsed.TotalSeconds, 1)

        # Parse per-crate results
        # cargo test outputs lines like:
        #   running N tests 
        #   test ... ok/FAILED/ignored
        #   test result: ok/FAILED. P passed; F failed; I ignored; ...
        # and between crates: "Running unittests (... crate_name ...)"

        $crateResults = [ordered]@{}
        $currentCrate = "(unknown)"
        $failedTests = @()
        $failureCapture = $false
        $failureBlock = ""
        $failureTestName = ""

        $totalPassed = 0
        $totalFailed = 0
        $totalIgnored = 0

        foreach ($line in $testOutput -split "`n") {
            $trimmed = $line.Trim()

            # Detect crate name from deps path — handles both same-line and split-line output
            # e.g. "Running unittests ... (target\debug\deps\claude_api-hashhere.exe)"
            # or just "(target\debug\deps\claude_api-hashhere.exe)" on its own line
            if ($trimmed -match 'deps[/\\](\w+)-[0-9a-f]+') {
                $currentCrate = $Matches[1]
            } elseif ($trimmed -match '^Doc-tests\s+(\w+)') {
                $currentCrate = $Matches[1]
            }

            # Detect "test result:" line
            if ($trimmed -match '^test result:.*?(\d+) passed.*?(\d+) failed.*?(\d+) ignored') {
                $p = [int]$Matches[1]
                $f = [int]$Matches[2]
                $ig = [int]$Matches[3]
                $totalPassed += $p
                $totalFailed += $f
                $totalIgnored += $ig

                if ($crateResults.Contains($currentCrate)) {
                    $crateResults[$currentCrate].Passed += $p
                    $crateResults[$currentCrate].Failed += $f
                    $crateResults[$currentCrate].Ignored += $ig
                } else {
                    $crateResults[$currentCrate] = @{ Passed = $p; Failed = $f; Ignored = $ig }
                }
            }

            # Capture individual test results (for ShowAll or failures)
            if ($trimmed -match '^test (.+) \.\.\. (ok|FAILED|ignored)') {
                $testName = $Matches[1]
                $testResult = $Matches[2]
                if ($ShowAll) {
                    $color = switch ($testResult) {
                        "ok"      { "Green" }
                        "FAILED"  { "Red" }
                        "ignored" { "DarkGray" }
                    }
                    Write-Host "    $testResult  $testName" -ForegroundColor $color
                }
                if ($testResult -eq "FAILED") {
                    $failedTests += "$currentCrate::$testName"
                }
            }

            # Capture failure details (between ---- name stdout ---- markers)
            if ($trimmed -match '^---- (.+) stdout ----') {
                $failureCapture = $true
                $failureTestName = $Matches[1]
                $failureBlock = ""
            } elseif ($failureCapture -and $trimmed -match '^----$') {
                # Not actually end marker — could be just separator
            } elseif ($failureCapture -and ($trimmed -match '^failures::' -or $trimmed -match '^test result:')) {
                $failureCapture = $false
                if ($failureBlock.Trim()) {
                    $failedTests += @{ Name = $failureTestName; Detail = $failureBlock.Trim() }
                }
            } elseif ($failureCapture) {
                $failureBlock += "$line`n"
            }
        }

        # Print per-crate summary table
        Write-Host ""
        Write-Host "  Per-crate results:" -ForegroundColor Cyan
        Write-Host ("  {0,-24} {1,8} {2,8} {3,8}" -f "Crate", "Passed", "Failed", "Ignored")
        Write-Host ("  {0,-24} {1,8} {2,8} {3,8}" -f "------------------------", "------", "------", "-------")

        foreach ($crate in $crateResults.Keys) {
            $r = $crateResults[$crate]
            $color = if ($r.Failed -gt 0) { "Red" } else { "Green" }
            Write-Host ("  {0,-24} {1,8} {2,8} {3,8}" -f $crate, $r.Passed, $r.Failed, $r.Ignored) -ForegroundColor $color
        }

        Write-Host ("  {0,-24} {1,8} {2,8} {3,8}" -f "========================", "======", "======", "=======")
        $totalColor = if ($totalFailed -gt 0) { "Red" } else { "Green" }
        Write-Host ("  {0,-24} {1,8} {2,8} {3,8}" -f "TOTAL", $totalPassed, $totalFailed, $totalIgnored) -ForegroundColor $totalColor

        if ($testExit -eq 0 -and $totalFailed -eq 0) {
            Write-Host ""
            Write-Pass "All tests passed (${elapsed}s) - $totalPassed passed, $totalFailed failed, $totalIgnored ignored"
            $stageResults += @{ Name = "test"; Pass = $true; Time = $elapsed }
        } else {
            Write-Host ""
            Write-Fail "Tests failed (${elapsed}s) - $totalPassed passed, $totalFailed failed, $totalIgnored ignored"

            # Print failure details
            if ($failedTests.Count -gt 0) {
                Write-Host ""
                Write-Host "  Failed tests:" -ForegroundColor Red
                foreach ($ft in $failedTests) {
                    if ($ft -is [hashtable]) {
                        Write-Host "    - $($ft.Name)" -ForegroundColor Red
                        Write-Host "      $($ft.Detail)" -ForegroundColor DarkGray
                    } else {
                        Write-Host "    - $ft" -ForegroundColor Red
                    }
                }
            }

            $stageResults += @{ Name = "test"; Pass = $false; Time = $elapsed }
            $overallPass = $false
            if ($StopOnFail) { throw "Stopped at cargo test" }
        }

        # ── Stage 4: integration tests (informational; non-blocking) ──

        Write-StageHeader 4 $totalStages "cargo test --test integration"
        $integrationPath = Join-Path $ProjectRoot 'tests\integration\mod.rs'
        if (-not (Test-Path $integrationPath)) {
            Write-Warn "No integration tests found, skipping"
            $stageResults += @{ Name = "integration"; Pass = $true; Time = 0 }
        } else {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            $intOutput = cargo test --test integration 2>&1 | Out-String
            $intExit = $LASTEXITCODE
            $sw.Stop()
            $elapsed = [math]::Round($sw.Elapsed.TotalSeconds, 1)

            if ($intExit -eq 0) {
                Write-Pass "Integration tests passed (${elapsed}s)"
                $stageResults += @{ Name = "integration"; Pass = $true; Time = $elapsed }
            } else {
                Write-Warn "Integration tests failed (${elapsed}s); continuing (non-blocking stage)"
                $failLines = $intOutput -split "`n" | Where-Object {
                    $_ -match 'FAILED|failures:|panicked at|test result: FAILED'
                } | Select-Object -First 20
                foreach ($fl in $failLines) {
                    Write-Host "    $fl" -ForegroundColor DarkYellow
                }
                $stageResults += @{ Name = "integration"; Pass = $true; Time = $elapsed }
            }
        }
    }

} catch {
    if ($_.Exception.Message -notmatch '^Stopped at') {
        Write-Host ""
        Write-Fail "Unexpected error: $($_.Exception.Message)"
        $overallPass = $false
    }
} finally {
    Pop-Location
}

# ── Final Summary ─────────────────────────────────────────

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
foreach ($s in $stageResults) {
    $icon = if ($s.Pass) { "[PASS]" } else { "[FAIL]" }
    $color = if ($s.Pass) { "Green" } else { "Red" }
    Write-Host ("  {0,6}  {1,-20} {2,6}s" -f $icon, $s.Name, $s.Time) -ForegroundColor $color
}

$totalTime = ($stageResults | ForEach-Object { $_.Time } | Measure-Object -Sum).Sum
Write-Host ""
if ($overallPass) {
    Write-Host "=== ALL STAGES PASSED === (total: ${totalTime}s)" -ForegroundColor Green
    exit 0
} else {
    Write-Host "=== SOME STAGES FAILED === (total: ${totalTime}s)" -ForegroundColor Red
    exit 1
}
