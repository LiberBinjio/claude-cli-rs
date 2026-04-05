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
$CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$RustupExe = Join-Path $CargoBin 'rustup.exe'
$CargoExe = Join-Path $CargoBin 'cargo.exe'
$RustcExe = Join-Path $CargoBin 'rustc.exe'
$LlvmBinCandidates = @(
    'C:\Program Files\LLVM\bin',
    (Join-Path $env:LOCALAPPDATA 'Programs\LLVM\bin')
)

function Add-CargoBinToPath {
    if (Test-Path $CargoBin) {
        $pathEntries = $env:PATH -split ';'
        if ($pathEntries -notcontains $CargoBin) {
            $env:PATH = "$CargoBin;$env:PATH"
        }
    }
}

function Add-LlvmBinToPath {
    foreach ($llvmBin in $LlvmBinCandidates) {
        if ((Test-Path $llvmBin) -and ((Join-Path $llvmBin 'clang.exe') | Test-Path)) {
            $pathEntries = $env:PATH -split ';'
            if ($pathEntries -notcontains $llvmBin) {
                $env:PATH = "$llvmBin;$env:PATH"
            }
            return
        }
    }
}

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [Parameter(Mandatory = $true)]
        [string]$FailureMessage
    )

    $stdoutPath = Join-Path $env:TEMP ([guid]::NewGuid().ToString() + '.stdout.log')
    $stderrPath = Join-Path $env:TEMP ([guid]::NewGuid().ToString() + '.stderr.log')

    try {
        $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -NoNewWindow -Wait -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
        $stdout = if (Test-Path $stdoutPath) { Get-Content $stdoutPath -Raw } else { '' }
        $stderr = if (Test-Path $stderrPath) { Get-Content $stderrPath -Raw } else { '' }

        if ($process.ExitCode -ne 0) {
            $detail = ($stderr, $stdout | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join [Environment]::NewLine
            throw ($FailureMessage + [Environment]::NewLine + $detail).Trim()
        }

        return @{ Stdout = $stdout; Stderr = $stderr }
    } finally {
        Remove-Item $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
    }
}

function Invoke-NativeCommandStreaming {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [Parameter(Mandatory = $true)]
        [string]$FailureMessage
    )

    $oldPref = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $lines = @()
    try {
        & $FilePath @Arguments 2>&1 | ForEach-Object {
            $line = $_.ToString()
            $lines += $line
            if (-not [string]::IsNullOrWhiteSpace($line)) {
                Write-Host "  $line" -ForegroundColor DarkGray
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw ($FailureMessage + [Environment]::NewLine + ($lines -join [Environment]::NewLine)).Trim()
        }
    } finally {
        $ErrorActionPreference = $oldPref
    }
}

function Get-HostArchitecture {
    # Returns Arm64 or X64 in a way that works on both PowerShell 5.1 and Core.
    try {
        $archObj = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
        $archText = $archObj.ToString()
        if ($archText -eq 'Arm64') { return 'Arm64' }
        if ($archText -eq 'X64') { return 'X64' }
    } catch {
        # Fallback for Windows PowerShell 5.1
    }

    $envProcArch = $env:PROCESSOR_ARCHITECTURE
    if ($envProcArch -eq 'ARM64') { return 'Arm64' }
    if ($envProcArch -eq 'AMD64') { return 'X64' }

    throw "Unsupported Windows architecture: $envProcArch"
}

function Get-RustupInitUrl {
    $arch = Get-HostArchitecture
    switch ($arch) {
        'Arm64' { return 'https://win.rustup.rs/aarch64' }
        'X64' { return 'https://win.rustup.rs/x86_64' }
        default { throw "Unsupported Windows architecture: $arch ($env:PROCESSOR_ARCHITECTURE)" }
    }
}

function Install-VsBuildToolsIfNeeded {
    $vsMsvcRoot = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC'
    $hasLinkOnPath = [bool](Get-Command link.exe -ErrorAction SilentlyContinue)
    $hasBuildToolsFiles = Test-Path $vsMsvcRoot

    if ($hasLinkOnPath -or $hasBuildToolsFiles) {
        if ($hasLinkOnPath) {
            Write-Host '[4/6] Visual Studio Build Tools already installed.' -ForegroundColor Green
        } else {
            Write-Host '[4/6] Visual Studio Build Tools already installed (not in current PATH).' -ForegroundColor Green
            Write-Host '  Open a Developer PowerShell or run vcvarsall.bat before cargo build if link.exe is not found.' -ForegroundColor Yellow
        }
        return
    }

    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) {
        Write-Host '[4/6] Visual Studio Build Tools missing.' -ForegroundColor Yellow
        Write-Host '  Install Visual Studio 2022 Build Tools with the Desktop development with C++ workload before running cargo build.' -ForegroundColor Yellow
        return
    }

    $arch = Get-HostArchitecture
    $toolComponent = if ($arch -eq 'Arm64') {
        'Microsoft.VisualStudio.Component.VC.Tools.ARM64'
    } else {
        'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
    }

    $override = @(
        '--wait',
        '--passive',
        '--norestart',
        '--add', 'Microsoft.VisualStudio.Workload.VCTools',
        '--add', $toolComponent,
        '--add', 'Microsoft.VisualStudio.Component.Windows11SDK.26100',
        '--includeRecommended'
    ) -join ' '

    Write-Host '[4/6] Installing Visual Studio Build Tools...' -ForegroundColor Yellow
    Write-Host '  Accept the UAC prompt if Windows asks for elevation.' -ForegroundColor Yellow
    Start-Process -FilePath $winget.Source -Verb RunAs -ArgumentList @(
        'install',
        '--id', 'Microsoft.VisualStudio.2022.BuildTools',
        '--exact',
        '--accept-package-agreements',
        '--accept-source-agreements',
        '--override', $override
    ) -Wait

    if (Get-Command link.exe -ErrorAction SilentlyContinue) {
        Write-Host '  Build Tools ready.' -ForegroundColor Green
    } else {
        Write-Host '  Build Tools install finished. Open a new terminal if cargo still cannot find link.exe.' -ForegroundColor Yellow
    }
}

function Install-ClangIfNeeded {
    if ((Get-HostArchitecture) -ne 'Arm64') {
        return
    }

    Add-LlvmBinToPath
    if (Get-Command clang.exe -ErrorAction SilentlyContinue) {
        Write-Host '[5/7] clang already installed.' -ForegroundColor Green
        return
    }

    $winget = Get-Command winget.exe -ErrorAction SilentlyContinue
    if (-not $winget) {
        Write-Host '[5/7] clang missing.' -ForegroundColor Yellow
        Write-Host '  Install LLVM (clang) before running cargo on Windows ARM64.' -ForegroundColor Yellow
        return
    }

    Write-Host '[5/7] Installing LLVM/clang...' -ForegroundColor Yellow
    Write-Host '  Accept the UAC prompt if Windows asks for elevation.' -ForegroundColor Yellow
    Start-Process -FilePath $winget.Source -Verb RunAs -ArgumentList @(
        'install',
        '--id', 'LLVM.LLVM',
        '--exact',
        '--accept-package-agreements',
        '--accept-source-agreements'
    ) -Wait

    Add-LlvmBinToPath
    if (Get-Command clang.exe -ErrorAction SilentlyContinue) {
        Write-Host '  clang ready.' -ForegroundColor Green
    } else {
        Write-Host '  LLVM install finished. Open a new terminal if clang is still not found.' -ForegroundColor Yellow
    }
}

Write-Host "=== Claude CLI (Rust) - Installation ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check/install rustup
Add-CargoBinToPath
Add-LlvmBinToPath

if (-not (Test-Path $RustupExe)) {
    Write-Host "[1/6] Installing Rust toolchain..." -ForegroundColor Yellow
    $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri (Get-RustupInitUrl) -OutFile $rustupInit -UseBasicParsing
    Invoke-NativeCommand -FilePath $rustupInit -Arguments @('-y', '--profile', 'minimal', '--default-toolchain', 'stable') -FailureMessage 'Rustup installation failed.' | Out-Null
    Add-CargoBinToPath
    if (-not (Test-Path $RustupExe)) {
        Write-Host "  ERROR: rustup not found after install. Add ~/.cargo/bin to PATH." -ForegroundColor Red
        exit 1
    }
    Write-Host "  Rust installed successfully." -ForegroundColor Green
} else {
    Write-Host "[1/6] rustup already installed." -ForegroundColor Green
}

# 2. Ensure stable toolchain
Write-Host "[2/6] Ensuring stable toolchain..." -ForegroundColor Yellow
Invoke-NativeCommand -FilePath $RustupExe -Arguments @('toolchain', 'install', 'stable', '--profile', 'minimal', '--no-self-update') -FailureMessage 'Failed to install the stable Rust toolchain.' | Out-Null
Invoke-NativeCommand -FilePath $RustupExe -Arguments @('default', 'stable') -FailureMessage 'Failed to set the default Rust toolchain to stable.' | Out-Null
Write-Host "  Stable toolchain ready." -ForegroundColor Green

# 3. Components
Write-Host "[3/6] Adding clippy + rustfmt..." -ForegroundColor Yellow
Invoke-NativeCommand -FilePath $RustupExe -Arguments @('component', 'add', 'clippy', 'rustfmt') -FailureMessage 'Failed to install clippy and rustfmt.' | Out-Null
Write-Host "  Components added." -ForegroundColor Green

# 4. Build tools
Install-VsBuildToolsIfNeeded

# 5. Clang for Windows ARM64
Install-ClangIfNeeded

# 6. Verify
Write-Host "[6/7] Verifying installation..." -ForegroundColor Yellow
Write-Host ""
Write-Host "  rustc : $(& $RustcExe --version)" -ForegroundColor Green
Write-Host "  cargo : $(& $CargoExe --version)" -ForegroundColor Green
Write-Host "  clippy: $(& $CargoExe clippy --version 2>&1)" -ForegroundColor Green
if ((Get-HostArchitecture) -eq 'Arm64') {
    $clangVersion = if (Get-Command clang.exe -ErrorAction SilentlyContinue) { & clang.exe --version | Select-Object -First 1 } else { 'not found' }
    Write-Host "  clang : $clangVersion" -ForegroundColor Green
}

# 7. Fetch dependencies
Write-Host ""
Write-Host "[7/7] Fetching dependencies..." -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    $env:CARGO_TERM_PROGRESS_WHEN = 'always'
    Invoke-NativeCommandStreaming -FilePath $CargoExe -Arguments @('fetch', '-v') -FailureMessage 'Failed to fetch cargo dependencies.'
    Write-Host "  Dependencies fetched." -ForegroundColor Green
} finally {
    Remove-Item Env:CARGO_TERM_PROGRESS_WHEN -ErrorAction SilentlyContinue
    Pop-Location
}

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host "Next step: cargo run -p claude_cli -- --help" -ForegroundColor Cyan
