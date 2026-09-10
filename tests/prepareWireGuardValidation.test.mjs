import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const wireGuardHelperPath = path.resolve("scripts/prepare-wireguard.helpers.ps1");
const hasPowerShellCore = () => {
  const result = spawnSync("pwsh", ["-NoProfile", "-Command", "exit 0"], {
    stdio: "ignore",
  });

  return !result.error && result.status === 0;
};
const canRunWireGuardWindowsContract =
  process.platform === "win32" && hasPowerShellCore();

test("WireGuard preparation accepts the official signer identity", (t) => {
  if (!canRunWireGuardWindowsContract) {
    t.skip("WireGuard Windows contract requires win32 and pwsh");
  }

  const tempDirectory = fs.mkdtempSync(
    path.join(os.tmpdir(), "ci-wireguard-signer-identity-"),
  );
  const tempScriptPath = path.join(tempDirectory, "signer-identity.ps1");
  fs.writeFileSync(
    tempScriptPath,
    `. '${wireGuardHelperPath.replace(/'/g, "''")}'
$trustedSubject = Test-OfficialWireGuardSignerSubject -SignatureLike ([pscustomobject]@{ Subject = 'CN=WireGuard LLC, O=WireGuard LLC' })
$legacySubject = Test-OfficialWireGuardSignerSubject -SignatureLike ([pscustomobject]@{ Subject = 'CN=Jason A. Donenfeld' })
$trustedFallback = Test-OfficialWireGuardSignerSubject -SignatureLike ([pscustomobject]@{ SimpleName = 'WireGuard LLC'; Subject = 'WireGuard LLC' })
$mismatch = Test-OfficialWireGuardSignerSubject -SignatureLike ([pscustomobject]@{ SimpleName = 'WireGuard LLC'; Subject = 'CN=AAA Certificate Services, O=WireGuard LLC' })
Write-Output "$trustedSubject,$legacySubject,$trustedFallback,$mismatch"
`,
    "utf8",
  );

  try {
    const output = execFileSync("pwsh", ["-NoProfile", "-File", tempScriptPath], {
      encoding: "utf8",
    }).trim();

    assert.equal(output, "True,True,True,False");
  } finally {
    fs.rmSync(tempDirectory, { force: true, recursive: true });
  }
});

test("WireGuard preparation accepts valid MSI signatures but rejects non-official executable publishers", (t) => {
  if (!canRunWireGuardWindowsContract) {
    t.skip("WireGuard Windows contract requires win32 and pwsh");
  }

  const tempDirectory = fs.mkdtempSync(
    path.join(os.tmpdir(), "ci-wireguard-validation-"),
  );
  const tempScriptPath = path.join(tempDirectory, "validate-wireguard.ps1");
  fs.writeFileSync(
    tempScriptPath,
    `. '${wireGuardHelperPath.replace(/'/g, "''")}'
function Get-CodeSignature {
    param([Parameter(Mandatory = $true)][string]$Path)

    switch ($Path) {
        'msi-valid' {
            return [pscustomobject]@{
                Status = 'Valid'
                SignerCertificate = [pscustomobject]@{
                    Subject = 'CN=AAA Certificate Services, O=AAA Certificate Services'
                }
                PublisherIdentity = 'AAA Certificate Services'
            }
        }
        'exe-valid-subject' {
            return [pscustomobject]@{
                Status = 'Valid'
                SignerCertificate = [pscustomobject]@{
                    Subject = 'CN=WireGuard LLC, O=WireGuard LLC'
                }
            }
        }
        'exe-valid-fallback' {
            return [pscustomobject]@{
                Status = 'Valid'
                SignerCertificate = [pscustomobject]@{
                    Subject = 'WireGuard LLC'
                }
                PublisherIdentity = 'WireGuard LLC'
            }
        }
        'exe-subject-mismatch' {
            return [pscustomobject]@{
                Status = 'Valid'
                SignerCertificate = [pscustomobject]@{
                    Subject = 'CN=AAA Certificate Services, O=WireGuard LLC'
                }
                PublisherIdentity = 'WireGuard LLC'
            }
        }
        default {
            throw "Unexpected path: $Path"
        }
    }
}

Assert-AuthenticodeSignature -Path 'msi-valid' -Label 'Downloaded WireGuard MSI' | Out-Null
Write-Output 'msi-valid-pass'
Assert-AuthenticodeSignature -Path 'exe-valid-subject' -Label 'WireGuard executable' -RequireOfficialWireGuardSigner | Out-Null
Write-Output 'exe-valid-subject-pass'
Assert-AuthenticodeSignature -Path 'exe-valid-fallback' -Label 'WireGuard executable' -RequireOfficialWireGuardSigner | Out-Null
Write-Output 'exe-valid-fallback-pass'

try {
    Assert-AuthenticodeSignature -Path 'exe-subject-mismatch' -Label 'WireGuard executable' -RequireOfficialWireGuardSigner | Out-Null
    Write-Output 'unexpected-pass'
}
catch {
    Write-Output $_.Exception.Message
}
`,
    "utf8",
  );

  try {
    const output = execFileSync("pwsh", ["-NoProfile", "-File", tempScriptPath], {
      encoding: "utf8",
    });

    assert.match(output, /msi-valid-pass/);
    assert.match(output, /exe-valid-subject-pass/);
    assert.match(output, /exe-valid-fallback-pass/);
    assert.doesNotMatch(output, /unexpected-pass/);
    assert.match(output, /WireGuard executable is Authenticode-signed but not by a trusted official WireGuard publisher/);
  } finally {
    fs.rmSync(tempDirectory, { force: true, recursive: true });
  }
});

test("WireGuard preparation reports when PowerShell signature tooling is unavailable", (t) => {
  if (!canRunWireGuardWindowsContract) {
    t.skip("WireGuard Windows contract requires win32 and pwsh");
  }

  const tempDirectory = fs.mkdtempSync(
    path.join(os.tmpdir(), "ci-wireguard-missing-signature-tooling-"),
  );
  const tempScriptPath = path.join(tempDirectory, "missing-signature-tooling.ps1");
  fs.writeFileSync(
    tempScriptPath,
    `. '${wireGuardHelperPath.replace(/'/g, "''")}'
function Import-Module {
    throw 'module unavailable'
}

function Get-Command {
    param()
    return $null
}

\${env:ProgramFiles(x86)} = ''

try {
    Get-CodeSignature -Path 'wireguard.exe' | Out-Null
    Write-Output 'unexpected-pass'
}
catch {
    Write-Output $_.Exception.Message
}
`,
    "utf8",
  );

  try {
    const output = execFileSync("pwsh", ["-NoProfile", "-File", tempScriptPath], {
      encoding: "utf8",
    });

    assert.doesNotMatch(output, /unexpected-pass/);
    assert.match(output, /Microsoft\.PowerShell\.Security and signtool\.exe are unavailable/);
  } finally {
    fs.rmSync(tempDirectory, { force: true, recursive: true });
  }
});
