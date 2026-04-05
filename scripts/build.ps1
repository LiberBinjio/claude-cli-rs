#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build claude-cli-rs in debug or release mode.
.PARAMETER Release
    Build in release mode (optimized).
.EXAMPLE
    .\scripts\build.ps1
    .\scripts\build.ps1 -Release
#>
[CmdletBinding()]
param([switch]$Release)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$LlvmBinCandidates = @(
    'C:\Program Files\LLVM\bin',
    (Join-Path $env:LOCALAPPDATA 'Programs\LLVM\bin')
)

if (Test-Path $CargoBin) {
    $pathEntries = $env:PATH -split ';'
    if ($pathEntries -notcontains $CargoBin) {
        $env:PATH = "$CargoBin;$env:PATH"
    }
}

foreach ($llvmBin in $LlvmBinCandidates) {
    if ((Test-Path $llvmBin) -and ((Join-Path $llvmBin 'clang.exe') | Test-Path)) {
        $pathEntries = $env:PATH -split ';'
        if ($pathEntries -notcontains $llvmBin) {
            $env:PATH = "$llvmBin;$env:PATH"
        }
        break
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'ERROR: cargo not found. Run .\scripts\install.ps1 first, or reopen your terminal after installing Rust.' -ForegroundColor Red
    exit 1
}

function Invoke-CargoBuild {
    param([string[]]$CargoArgs)

    $oldPref = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $lines = @()
    try {
        & cargo @CargoArgs 2>&1 | ForEach-Object {
            $line = $_.ToString()
            $lines += $line
            Write-Host "  $line" -ForegroundColor DarkGray
        }
        return @{ ExitCode = $LASTEXITCODE; Output = ($lines -join "`n") }
    } finally {
        $ErrorActionPreference = $oldPref
    }
}

Write-Host "=== Claude CLI (Rust) - Build ===" -ForegroundColor Cyan

Push-Location $ProjectRoot
try {
    if ($Release) {
        Write-Host "Building release..." -ForegroundColor Yellow
        $result = Invoke-CargoBuild -CargoArgs @('build', '--release')
        $bin = Join-Path $ProjectRoot 'target\release\claude.exe'
        $altBin = Join-Path $ProjectRoot 'target\release\claude-cli-rs.exe'
    } else {
        Write-Host "Building debug..." -ForegroundColor Yellow
        $result = Invoke-CargoBuild -CargoArgs @('build')
        $bin = Join-Path $ProjectRoot 'target\debug\claude.exe'
        $altBin = Join-Path $ProjectRoot 'target\debug\claude-cli-rs.exe'
    }

    if ($result.ExitCode -ne 0) {
        Write-Host "" 
        Write-Host "BUILD FAILED" -ForegroundColor Red
        if ($result.Output -match 'failed to find tool "clang"') {
            Write-Host "Hint: clang is missing. On Windows ARM64, run .\scripts\install.ps1 to install LLVM." -ForegroundColor Yellow
        }
        if ($result.Output -match 'link\.exe`? not found') {
            Write-Host "Hint: MSVC linker is missing. Run .\scripts\install.ps1 to install Visual Studio Build Tools." -ForegroundColor Yellow
        }
        if ($result.Output -match 'os error 5') {
            Write-Host "Hint: executable may be locked by a running process. Close claude.exe and retry." -ForegroundColor Yellow
        }
        exit $result.ExitCode
    }

    if (-not (Test-Path $bin) -and (Test-Path $altBin)) {
        $bin = $altBin
    }

    if (Test-Path $bin) {
        $size = (Get-Item $bin).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        Write-Host ""
        Write-Host "  Binary: $bin" -ForegroundColor Green
        Write-Host "  Size:   $sizeMB MB" -ForegroundColor Green
    } else {
        Write-Host "  Warning: Binary not found at expected path." -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Build complete!" -ForegroundColor Green
} finally {
    Pop-Location
}
