# CinaVault Premium Windows Installer Build Script
# Builds the production web app, validates the Rust/Tauri side, and creates Windows installer bundles.

param(
    [switch]$SkipTests,
    [switch]$NoDesktopCopy,
    [switch]$NoOpenDesktop
)
    Set-Location $PSScriptRoot\..
    $RepoRoot = "$PSScriptRoot\.."

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"


function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Require-Command {
    param(
        [string]$Name,
        [string]$InstallHint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found. $InstallHint"
    }
}

function Show-Npm-Debug-Log {
    $CandidateRoots = @()
    if ($env:npm_config_cache) {
        $CandidateRoots += (Join-Path $env:npm_config_cache "_logs")
    }
    $CandidateRoots += "C:\npm\cache\_logs"
    $CandidateRoots += (Join-Path $env:LOCALAPPDATA "npm-cache\_logs")

    $LatestLog = $null
    foreach ($Root in $CandidateRoots | Where-Object { $_ -and (Test-Path $_) }) {
        $Log = Get-ChildItem -Path $Root -Filter "*.log" -File -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($Log -and (-not $LatestLog -or $Log.LastWriteTime -gt $LatestLog.LastWriteTime)) {
            $LatestLog = $Log
        }
    }

    if ($LatestLog) {
        Write-Host ""
        Write-Host "Latest npm debug log: $($LatestLog.FullName)" -ForegroundColor Yellow
        Get-Content -Path $LatestLog.FullName -Tail 160
    } else {
        Write-Host "No npm debug log was found." -ForegroundColor Yellow
    }
}

function Show-Tauri-Diagnostics {
    Write-Host ""
    Write-Host "Tauri diagnostic info" -ForegroundColor Yellow
    & npx tauri info
    $InfoExitCode = $LASTEXITCODE
    if ($InfoExitCode -ne 0) {
        Write-Host "tauri info exited with code $InfoExitCode" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Recent bundle directory contents" -ForegroundColor Yellow
    $BundleRoot = Join-Path $RepoRoot "src-tauri\target\release\bundle"
    if (Test-Path $BundleRoot) {
        Get-ChildItem -Path $BundleRoot -Recurse -Force | Select-Object -First 200 | ForEach-Object {
            Write-Host $_.FullName
        }
    } else {
        Write-Host "Bundle directory does not exist yet: $BundleRoot"
    }
}

function Invoke-Checked {
    param(
        [string]$Command,
        [string[]]$Arguments,
        [switch]$TauriDiagnostics
    )

    & $Command @Arguments
    $ExitCode = $LASTEXITCODE
    if ($ExitCode -ne 0) {
        $ArgumentText = $Arguments -join ' '
        if ($TauriDiagnostics) {
            Show-Tauri-Diagnostics
        }
        if ($Command -eq "npm" -or $Command -eq "npx") {
            Show-Npm-Debug-Log
        }
        throw ('Command failed with exit code {0}: {1} {2}' -f $ExitCode, $Command, $ArgumentText)
    }
}

Write-Host "CinaVault Premium Windows Installer Build" -ForegroundColor Magenta
Write-Host "Repository: $RepoRoot"

Write-Step "Checking required tools"
Require-Command "node" "Install Node.js LTS."
Require-Command "npm" "Install Node.js LTS."
Require-Command "npx" "Install Node.js LTS."
Require-Command "cargo" "Install Rust."
Require-Command "rustc" "Install Rust."

if (Test-Path (Join-Path $RepoRoot "node_modules")) {
    Write-Step "Using preinstalled JavaScript dependencies"
} else {
    Write-Step "Installing JavaScript dependencies from the current fresh manifest"
    Invoke-Checked "npm" @("install", "--legacy-peer-deps", "--loglevel", "verbose")
}

Write-Step "Running TypeScript simulation build"
Invoke-Checked "npx" @("tsc", "-p", "tsconfig.build.json")

Write-Step "Running Vite production build"
Invoke-Checked "npx" @("vite", "build")

if (-not $SkipTests) {
    Write-Step "Running JavaScript surface regression tests"
    Invoke-Checked "npm" @("run", "test:build140")

    Write-Step "Running Rust compile check"
    Invoke-Checked "cargo" @("check", "--manifest-path", "src-tauri/Cargo.toml")

    Write-Step "Running scanner ingestion regression tests"
    Invoke-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml", "scanner::tests", "--", "--nocapture")

    Write-Step "Running metadata poster posting regression test"
    Invoke-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml", "metadata_posting_tests", "--", "--nocapture")

    Write-Step "Running PGMA bridge tests"
    Invoke-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml", "pgma_bridge", "--", "--nocapture")

    Write-Step "Running PGMA plugin deployer tests"
    Invoke-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml", "plugins::tests", "--", "--nocapture")
} else {
    Write-Step "Skipping tests because -SkipTests was supplied"
}

Write-Step "Building Windows installer with Tauri"
$env:RUST_BACKTRACE = "full"
$env:TAURI_DEBUG = "true"
Invoke-Checked "npx" @("tauri", "build") -TauriDiagnostics

Write-Step "Finding installer outputs"
$Installers = @()
$BundleRoot = Join-Path $RepoRoot "src-tauri\target\release\bundle"
if (Test-Path $BundleRoot) {
    $Installers = Get-ChildItem -Path $BundleRoot -Recurse -File | Where-Object {
        $_.Extension -in ".exe", ".msi", ".zip"
    }
}

if (-not $Installers -or $Installers.Count -eq 0) {
    throw "No installer artifacts were produced under $BundleRoot."
}

Write-Host "Installer artifacts:" -ForegroundColor Green
foreach ($Installer in $Installers) {
    Write-Host " - $($Installer.FullName)"
}

if (-not $NoDesktopCopy) {
    $Desktop = [Environment]::GetFolderPath("Desktop")
    $OutDir = Join-Path $Desktop "CinaVault-Premium-Installer"
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

    foreach ($Installer in $Installers) {
        Copy-Item -Path $Installer.FullName -Destination $OutDir -Force
    }

    Write-Host "Copied installers to: $OutDir" -ForegroundColor Green
}

Write-Host ""
Write-Host "Build complete." -ForegroundColor Green
