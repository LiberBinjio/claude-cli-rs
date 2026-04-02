#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Full CI pipeline: check -> clippy -> test -> release build.
.DESCRIPTION
    Runs all CI stages sequentially, stops on first failure,
    prints timing for each stage, and exits with appropriate code.
.EXAMPLE
    .\scripts\ci.ps1
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$ProjectRoot = Split-Path -Parent $PSScriptRoot

function Invoke-CiStage {
    param([string]$Cmd)

    $oldPref = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $lines = @()
    try {
        & cmd /c "$Cmd 2>&1" | ForEach-Object {
            $line = $_.ToString()
            $lines += $line
        }
        return @{ ExitCode = $LASTEXITCODE; Output = ($lines -join "`n") }
    } finally {
        $ErrorActionPreference = $oldPref
    }
}

Write-Host '=== Claude CLI (Rust) - CI Pipeline ===' -ForegroundColor Cyan
Write-Host "  Project: $ProjectRoot" -ForegroundColor DarkGray
Write-Host ''

$stages = @(
    @{ Name = 'cargo check --workspace';      Cmd = 'cargo check --workspace' },
    @{ Name = 'cargo clippy (deny warnings)'; Cmd = 'cargo clippy --workspace -- -D warnings' },
    @{ Name = 'cargo test --workspace --exclude claude-cli-rs'; Cmd = 'cargo test --workspace --exclude claude-cli-rs' },
    @{ Name = 'cargo build --release';        Cmd = 'cargo build --release' }
)

$totalStages = $stages.Count
$passed = 0
$failed = 0
$results = @()
$ciStart = [System.Diagnostics.Stopwatch]::StartNew()

Push-Location $ProjectRoot
try {
    for ($i = 0; $i -lt $stages.Count; $i++) {
        $stage = $stages[$i]
        $num = $i + 1
        Write-Host "[$num/$totalStages] $($stage.Name)..." -ForegroundColor Yellow

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $run = Invoke-CiStage -Cmd $stage.Cmd
        $sw.Stop()
        $elapsed = [math]::Round($sw.Elapsed.TotalSeconds, 1)

        $results += @{
            Stage   = $stage.Name
            Seconds = $elapsed
            Success = ($run.ExitCode -eq 0)
        }

        if ($run.ExitCode -eq 0) {
            Write-Host "  PASSED (${elapsed}s)" -ForegroundColor Green
            $passed++
        } else {
            Write-Host "  FAILED (${elapsed}s)" -ForegroundColor Red
            $errorLines = ($run.Output -split "`n" | Where-Object { $_.Trim() -ne '' } | Select-Object -Last 30)
            foreach ($line in $errorLines) {
                Write-Host "    $line" -ForegroundColor DarkRed
            }
            $failed++
            Write-Host ''
            Write-Host "CI FAILED at stage: $($stage.Name)" -ForegroundColor Red
            break
        }
    }
} finally {
    Pop-Location
}

$ciStart.Stop()
$totalTime = [math]::Round($ciStart.Elapsed.TotalSeconds, 1)

Write-Host ''
Write-Host '=== CI Summary ===' -ForegroundColor Cyan

foreach ($r in $results) {
    $status = if ($r.Success) { 'PASS' } else { 'FAIL' }
    $color = if ($r.Success) { 'Green' } else { 'Red' }
    $line = '  {0,-40} {1,6}s  [{2}]' -f $r.Stage, $r.Seconds, $status
    Write-Host $line -ForegroundColor $color
}

$reached = $results.Count
if ($reached -lt $totalStages) {
    for ($j = $reached; $j -lt $totalStages; $j++) {
        $line = '  {0,-40} {1,6}   [{2}]' -f $stages[$j].Name, '-', 'SKIP'
        Write-Host $line -ForegroundColor DarkGray
    }
}

Write-Host ''
$summaryColor = if ($failed -eq 0) { 'Green' } else { 'Red' }
Write-Host "  Total: ${totalTime}s | Passed: $passed/$totalStages | Failed: $failed" -ForegroundColor $summaryColor

if ($failed -eq 0) {
    Write-Host ''
    Write-Host '  All CI stages passed!' -ForegroundColor Green
    exit 0
}

exit 1
