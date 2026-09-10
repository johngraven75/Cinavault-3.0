[CmdletBinding()]
param()

Set-StrictMode -Version Latest

$script:OfficialWireGuardSignerNames = @(
    'WireGuard',
    'WireGuard LLC',
    'Jason A. Donenfeld'
)

$script:OfficialWireGuardSignerSubjects = @(
    'CN=WireGuard LLC, O=WireGuard LLC',
    'CN=Jason A. Donenfeld'
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

function Get-NormalizedSignerSubject {
    param($SignerCertificate)

    if ($null -eq $SignerCertificate -or [string]::IsNullOrWhiteSpace($SignerCertificate.Subject)) {
        return $null
    }

    try {
        return ([System.Security.Cryptography.X509Certificates.X500DistinguishedName]::new($SignerCertificate.Subject)).Name
    }
    catch {
        return $null
    }
}

function Test-OfficialWireGuardSignerSubject {
    param($SignerCertificate)

    $normalizedSubject = Get-NormalizedSignerSubject -SignerCertificate $SignerCertificate
    if (-not [string]::IsNullOrWhiteSpace($normalizedSubject)) {
        return $script:OfficialWireGuardSignerSubjects -contains $normalizedSubject
    }

    $simpleName = Get-SignerSimpleName -SignerCertificate $SignerCertificate

    return $script:OfficialWireGuardSignerNames -contains $simpleName
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
