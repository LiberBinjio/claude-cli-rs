#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Install Rust toolchain and project dependencies for claude-cli-rs.
.DESCRIPTION
    Checks for rustup, installs it if missing, ensures stable toolchain
    with clippy + rustfmt, and fetches all cargo dependencies.
.EXAMPLE
    .\scripts\install.ps1
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ProjectRoot = Split-Path -Parent $PSScriptRoot

Write-Host "=== Claude CLI (Rust) - Installation ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check/install rustup
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "[1/5] Installing Rust toolchain..." -ForegroundColor Yellow
    $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit -UseBasicParsing
    & $rustupInit -y --default-toolchain stable
    # Refresh PATH
    $cargobin = Join-Path $env:USERPROFILE ".cargo\bin"
    $env:PATH = "$cargobin;$env:PATH"
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        Write-Host "  ERROR: rustup not found after install. Add ~/.cargo/bin to PATH." -ForegroundColor Red
        exit 1
    }
    Write-Host "  Rust installed successfully." -ForegroundColor Green
} else {
    Write-Host "[1/5] rustup already installed." -ForegroundColor Green
}

# 2. Ensure stable toolchain
Write-Host "[2/5] Ensuring stable toolchain..." -ForegroundColor Yellow
rustup toolchain install stable --no-self-update 2>&1 | Out-Null
rustup default stable 2>&1 | Out-Null
Write-Host "  Stable toolchain ready." -ForegroundColor Green

# 3. Components
Write-Host "[3/5] Adding clippy + rustfmt..." -ForegroundColor Yellow
rustup component add clippy rustfmt 2>&1 | Out-Null
Write-Host "  Components added." -ForegroundColor Green

# 4. Verify
Write-Host "[4/5] Verifying installation..." -ForegroundColor Yellow
Write-Host ""
Write-Host "  rustc : $(rustc --version)" -ForegroundColor Green
Write-Host "  cargo : $(cargo --version)" -ForegroundColor Green
Write-Host "  clippy: $(cargo clippy --version 2>&1)" -ForegroundColor Green

# 5. Fetch dependencies
Write-Host ""
Write-Host "[5/5] Fetching dependencies..." -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    cargo fetch 2>&1 | Out-Null
    Write-Host "  Dependencies fetched." -ForegroundColor Green
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "Next step: .\scripts\build.ps1" -ForegroundColor Cyan
