[CmdletBinding()]
param(
    [string]$Destination,
    [string]$MsiUrl = 'https://download.wireguard.com/windows-client/wireguard-amd64-1.1.msi',
    [switch]$ForceDownload
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'The bundled WireGuard engine can only be prepared on Windows.'
}

if ([string]::IsNullOrWhiteSpace($Destination)) {
    $scriptPath = $MyInvocation.MyCommand.Path
    if ([string]::IsNullOrWhiteSpace($scriptPath)) {
        throw 'Unable to resolve the WireGuard preparation script directory.'
    }
    $scriptDirectory = Split-Path -Parent $scriptPath
    $Destination = Join-Path $scriptDirectory '..\src-tauri\tools\wireguard\wireguard.exe'
}

function Get-CodeSignature {
    param([Parameter(Mandatory = $true)][string]$Path)

    try {
        Import-Module Microsoft.PowerShell.Security -ErrorAction Stop
        return Get-AuthenticodeSignature -LiteralPath $Path
    }
    catch {
        $signatureToolCommand = Get-Command signtool.exe -ErrorAction SilentlyContinue | Select-Object -First 1
        $signatureToolPath = if ($signatureToolCommand) { $signatureToolCommand.Source } else { $null }
        if ([string]::IsNullOrWhiteSpace($signatureToolPath)) {
            $windowsKitsProgramFiles = [System.Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
            $windowsKitsRoot = if ($windowsKitsProgramFiles) { Join-Path $windowsKitsProgramFiles 'Windows Kits\10\bin' } else { $null }
            if ($windowsKitsRoot -and (Test-Path -LiteralPath $windowsKitsRoot)) {
                $signatureToolPath = Get-ChildItem -LiteralPath $windowsKitsRoot -Recurse -File -Filter signtool.exe -ErrorAction SilentlyContinue |
                    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
                    Sort-Object FullName -Descending |
                    Select-Object -First 1 -ExpandProperty FullName
            }
        }
        if ([string]::IsNullOrWhiteSpace($signatureToolPath)) {
            throw "Unable to validate the Authenticode signature for $Path because Microsoft.PowerShell.Security and signtool.exe are unavailable. $($_.Exception.Message)"
        }

        $output = & $signatureToolPath verify /pa /v $Path 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "signtool.exe could not validate the Authenticode signature for $Path. $($output -join [Environment]::NewLine)"
        }

        $issuedTo = ($output | Where-Object { $_ -match '^\s*(Issued to|Subject):\s*(.+)$' } | Select-Object -First 1)
        $subject = if ($issuedTo -and $issuedTo -match '^\s*(Issued to|Subject):\s*(.+)$') { $Matches[2] } else { $output -join ' ' }
        return [pscustomobject]@{
            Status = 'Valid'
            SignerCertificate = [pscustomobject]@{
                Subject = $subject
            }
        }
    }
}

function Test-OfficialWireGuardBinary {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }

    $item = Get-Item -LiteralPath $Path
    if ($item.Length -lt 1MB -or $item.VersionInfo.ProductName -notmatch '(?i)WireGuard') {
        return $false
    }

    $signature = Get-CodeSignature -Path $Path
    return $signature.Status -eq 'Valid' -and
        $null -ne $signature.SignerCertificate -and
        $signature.SignerCertificate.Subject -match '(?i)WireGuard'
}

function Assert-OfficialSignature {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $signature = Get-CodeSignature -Path $Path
    if ($signature.Status -ne 'Valid' -or
        $null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch '(?i)WireGuard') {
        throw "$Label does not have a valid WireGuard Authenticode signature. Status: $($signature.Status)"
    }
}

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
if (-not $ForceDownload -and (Test-OfficialWireGuardBinary -Path $destinationPath)) {
    $existingHash = (Get-FileHash -LiteralPath $destinationPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Host "Official WireGuard engine already prepared: $destinationPath"
    Write-Host "SHA-256: $existingHash"
    exit 0
}

$programFilesX86 = [System.Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
$installedCandidates = @(
    (Join-Path $env:ProgramFiles 'WireGuard\wireguard.exe')
)
if ($programFilesX86) {
    $installedCandidates += Join-Path $programFilesX86 'WireGuard\wireguard.exe'
}

$source = $null
if (-not $ForceDownload) {
    $source = $installedCandidates |
        Where-Object { Test-OfficialWireGuardBinary -Path $_ } |
        Select-Object -First 1
}

$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) "cinavault-wireguard-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null

try {
    if (-not $source) {
        $msiPath = Join-Path $temporaryRoot 'wireguard-amd64.msi'
        $extractionPath = Join-Path $temporaryRoot 'extracted'
        New-Item -ItemType Directory -Path $extractionPath -Force | Out-Null

        Write-Host "Downloading the official WireGuard MSI from $MsiUrl"
        Invoke-WebRequest -Uri $MsiUrl -OutFile $msiPath -UseBasicParsing
        Assert-OfficialSignature -Path $msiPath -Label 'Downloaded WireGuard MSI'

        $msiArguments = "/a `"$msiPath`" /qn TARGETDIR=`"$extractionPath`""
        $extract = Start-Process msiexec.exe -ArgumentList $msiArguments -Wait -PassThru
        if ($extract.ExitCode -ne 0) {
            throw "WireGuard MSI extraction failed with exit code $($extract.ExitCode)."
        }

        $source = Get-ChildItem -LiteralPath $extractionPath -Recurse -File -Filter wireguard.exe |
            Where-Object { Test-OfficialWireGuardBinary -Path $_.FullName } |
            Select-Object -First 1 -ExpandProperty FullName
        if (-not $source) {
            throw 'The signed official WireGuard executable was not found in the extracted MSI.'
        }
    }

    $stagedPath = Join-Path $temporaryRoot 'wireguard.exe'
    Copy-Item -LiteralPath $source -Destination $stagedPath -Force
    if (-not (Test-OfficialWireGuardBinary -Path $stagedPath)) {
        throw 'The staged WireGuard executable failed publisher, product, size, or signature validation.'
    }

    $destinationDirectory = Split-Path -Parent $destinationPath
    New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
    Copy-Item -LiteralPath $stagedPath -Destination $destinationPath -Force

    if (-not (Test-OfficialWireGuardBinary -Path $destinationPath)) {
        throw 'The copied WireGuard executable failed final validation.'
    }

    $hash = (Get-FileHash -LiteralPath $destinationPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Host "Prepared official WireGuard engine: $destinationPath"
    Write-Host "SHA-256: $hash"
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
