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

$officialWireGuardSignerNames = @(
    'WireGuard',
    'WireGuard LLC',
    'Jason A. Donenfeld'
)

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
                SimpleName = $subject
            }
        }
    }
}

function Get-SignerSimpleName {
    param($SignerCertificate)

    if ($null -eq $SignerCertificate) {
        return $null
    }

    if ($SignerCertificate.PSObject.Methods.Name -contains 'GetNameInfo') {
        try {
            $simpleName = $SignerCertificate.GetNameInfo(
                [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
                $false
            )
            if (-not [string]::IsNullOrWhiteSpace($simpleName)) {
                return $simpleName.Trim()
            }
        }
        catch {
        }
    }

    if ($SignerCertificate.PSObject.Properties.Name -contains 'SimpleName' -and
        -not [string]::IsNullOrWhiteSpace($SignerCertificate.SimpleName)) {
        return $SignerCertificate.SimpleName.Trim()
    }

    if (-not [string]::IsNullOrWhiteSpace($SignerCertificate.Subject)) {
        try {
            $distinguishedName = [System.Security.Cryptography.X509Certificates.X500DistinguishedName]::new($SignerCertificate.Subject)
            $decodedSubject = $distinguishedName.Decode(
                [System.Security.Cryptography.X509Certificates.X500DistinguishedNameFlags]::UseNewLines
            )
            $commonName = $decodedSubject -split '\r?\n' |
                Where-Object { $_ -match '^\s*CN=' } |
                Select-Object -First 1
            if ($commonName -and $commonName -match '^\s*CN=(.+)$') {
                return $Matches[1].Trim()
            }
        }
        catch {
        }
    }

    return $SignerCertificate.Subject
}

function Test-OfficialWireGuardSignerSubject {
    param($SignerCertificate)

    $simpleName = Get-SignerSimpleName -SignerCertificate $SignerCertificate

    return $officialWireGuardSignerNames -contains $simpleName
}

function Assert-AuthenticodeSignature {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label,
        [switch]$RequireOfficialWireGuardSigner
    )

    $signature = Get-CodeSignature -Path $Path
    $signerSubject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) {
        throw "$Label does not have a valid Authenticode signature. Status: $($signature.Status); signer: $signerSubject"
    }

    if ($RequireOfficialWireGuardSigner -and -not (Test-OfficialWireGuardSignerSubject -SignerCertificate $signature.SignerCertificate)) {
        throw "$Label is Authenticode-signed but not by a trusted official WireGuard publisher. Status: $($signature.Status); signer: $signerSubject"
    }

    return $signature
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

    try {
        Assert-AuthenticodeSignature -Path $Path -Label 'WireGuard executable' -RequireOfficialWireGuardSigner | Out-Null
        return $true
    }
    catch {
        return $false
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
        Assert-AuthenticodeSignature -Path $msiPath -Label 'Downloaded WireGuard MSI'

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
