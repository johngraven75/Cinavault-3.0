[CmdletBinding()]

param()

Set-StrictMode -Version Latest

$script:OfficialWireGuardSignerNames = @('wireguard llc','jason a. donenfeld')

function Get-SignerPublisherIdentity { param($SignerCertificate)

    if ($null -eq $SignerCertificate) { return $null }
    
    try { $name = $SignerCertificate.GetNameInfo([System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,$false); if (-not [string]::IsNullOrWhiteSpace($name)) { return $name.Trim() } } catch {}
    
    if ($SignerCertificate.PSObject.Properties.Name -contains 'SimpleName' -and $SignerCertificate.SimpleName) { return $SignerCertificate.SimpleName.Trim() }
    
    if ($SignerCertificate.Subject -match '(?i)(?:^|,)\s*CN=([^,]+)') { return $Matches[1].Trim() }
    
    return $SignerCertificate.Subject
    
}

function Get-CodeSignature { param([Parameter(Mandatory=$true)][string]$Path)

    Import-Module Microsoft.PowerShell.Security -ErrorAction Stop
    
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    
    [pscustomobject]@{ Status=[string]$signature.Status; SignerCertificate=$signature.SignerCertificate; PublisherIdentity=Get-SignerPublisherIdentity $signature.SignerCertificate }
    
}

function Test-OfficialWireGuardSignerSubject { param($SignatureLike)

    $cert = if ($SignatureLike.PSObject.Properties.Name -contains 'SignerCertificate') { $SignatureLike.SignerCertificate } else { $SignatureLike }
    
    $identity = if ($SignatureLike.PSObject.Properties.Name -contains 'PublisherIdentity') { [string]$SignatureLike.PublisherIdentity } else { Get-SignerPublisherIdentity $cert }
    
    if ($identity -and $script:OfficialWireGuardSignerNames -contains $identity.Trim().ToLowerInvariant()) { return $true }
    
    $subject = if ($cert) { [string]$cert.Subject } else { '' }
    
    foreach ($name in $script:OfficialWireGuardSignerNames) { if ($subject.ToLowerInvariant().Contains($name)) { return $true } }
    
    return $false
    
}

function Assert-AuthenticodeSignature { param([Parameter(Mandatory=$true)][string]$Path,[Parameter(Mandatory=$true)][string]$Label,[switch]$RequireOfficialWireGuardSigner)

    $signature=Get-CodeSignature $Path
    
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { throw "$Label does not have a valid Authenticode signature. Status: $($signature.Status); signer: $($signature.SignerCertificate.Subject)" }
    
    if ($RequireOfficialWireGuardSigner -and -not (Test-OfficialWireGuardSignerSubject $signature)) { throw "$Label is not signed by an official WireGuard publisher. Signer: $($signature.SignerCertificate.Subject)" }
    
    return $signature
    
}

function Test-OfficialWireGuardBinary { param([Parameter(Mandatory=$true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    
    $item=Get-Item $Path
    
    if ($item.Length -lt 1MB -or $item.VersionInfo.ProductName -notmatch '(?i)WireGuard') { return $false }
    
    try { Assert-AuthenticodeSignature $Path 'WireGuard executable' -RequireOfficialWireGuardSigner | Out-Null; return $true } catch { return $false }
    
}























