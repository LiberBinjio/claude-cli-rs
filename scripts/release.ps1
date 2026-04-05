#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build an optimized release binary and package it for distribution.
.DESCRIPTION
    Builds claude-cli-rs in release mode, copies the binary to dist/,
    creates a zip archive, and computes SHA256 checksum.
.EXAMPLE
    .\scripts\release.ps1
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $ProjectRoot 'dist'
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

function Invoke-CargoRelease {
    $oldPref = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $lines = @()
    try {
        & cargo build --release 2>&1 | ForEach-Object {
            $line = $_.ToString()
            $lines += $line
            Write-Host "  $line" -ForegroundColor DarkGray
        }
        return @{ ExitCode = $LASTEXITCODE; Output = ($lines -join "`n") }
    } finally {
        $ErrorActionPreference = $oldPref
    }
}

Write-Host '=== Claude CLI (Rust) - Release Build ===' -ForegroundColor Cyan

Write-Host '[1/4] Building release binary...' -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    $buildResult = Invoke-CargoRelease
    if ($buildResult.ExitCode -ne 0) {
        Write-Host 'FAILED: Release build failed' -ForegroundColor Red
        if ($buildResult.Output -match 'failed to find tool "clang"') {
            Write-Host 'Hint: clang is missing. On Windows ARM64, run .\scripts\install.ps1 to install LLVM.' -ForegroundColor Yellow
        }
        if ($buildResult.Output -match 'link\.exe`? not found') {
            Write-Host 'Hint: MSVC linker is missing. Run .\scripts\install.ps1 to install Visual Studio Build Tools.' -ForegroundColor Yellow
        }
        if ($buildResult.Output -match 'os error 5') {
            Write-Host 'Hint: executable may be locked by a running process. Close claude.exe and retry.' -ForegroundColor Yellow
        }
        exit $buildResult.ExitCode
    }
} finally {
    Pop-Location
}

$BinSrc = Join-Path $ProjectRoot 'target\release\claude.exe'
if (-not (Test-Path $BinSrc)) {
    $BinSrc = Join-Path $ProjectRoot 'target\release\claude-cli-rs.exe'
}
if (-not (Test-Path $BinSrc)) {
    Write-Host 'ERROR: Release binary not found in target\release\' -ForegroundColor Red
    Write-Host '  Looked for: claude.exe, claude-cli-rs.exe' -ForegroundColor Red
    exit 1
}

Write-Host '[2/4] Preparing dist...' -ForegroundColor Yellow
if (Test-Path $DistDir) { Remove-Item -Recurse -Force $DistDir }
New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

$BinDest = Join-Path $DistDir 'claude.exe'
Copy-Item $BinSrc $BinDest
$size = (Get-Item $BinDest).Length
$sizeMB = [math]::Round($size / 1MB, 2)
Write-Host "  Binary: $BinDest ($sizeMB MB)" -ForegroundColor Green

Write-Host '[3/4] Creating archive...' -ForegroundColor Yellow
$version = '0.1.0'
$arch = if ([Environment]::Is64BitOperatingSystem) { 'x86_64' } else { 'x86' }
$archiveName = "claude-cli-rs-v$version-windows-$arch.zip"
$archivePath = Join-Path $DistDir $archiveName

Compress-Archive -Path $BinDest -DestinationPath $archivePath -Force
$archiveSize = [math]::Round((Get-Item $archivePath).Length / 1MB, 2)
Write-Host "  Archive: $archivePath ($archiveSize MB)" -ForegroundColor Green

Write-Host '[4/4] Computing checksum...' -ForegroundColor Yellow
$hash = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLower()
$checksumFile = Join-Path $DistDir 'SHA256SUMS.txt'
"$hash  $archiveName" | Set-Content $checksumFile
Write-Host "  SHA256: $hash" -ForegroundColor Green

Write-Host ''
Write-Host "Release artifacts in: $DistDir" -ForegroundColor Cyan
Get-ChildItem $DistDir | ForEach-Object {
    $sizeKB = [math]::Round($_.Length / 1KB, 1)
    Write-Host "  $($_.Name) ($sizeKB KB)"
}
Write-Host ''
Write-Host 'Done!' -ForegroundColor Green
