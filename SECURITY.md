# Security policy

## Supported versions

| Version | Supported |
|---|---|
| The current release on this repository's default branch | Yes |
| Earlier builds | No |

If you are on an earlier build, upgrade to the current release before reporting — a fix, when confirmed, ships in the next build of this repo.

## Reporting a vulnerability

Please report security issues privately through GitHub: open this repository's **Security** tab and choose **Report a vulnerability**.
Do not open a public issue for a security problem.

Include the version (Settings > About, or the build string for this repo), the steps to reproduce the problem, and what an attacker could do with it.
You will get an acknowledgement, and a fix will ship in the next build once the report is confirmed. Please allow reasonable time for triage and remediation before any public disclosure.

## How this app protects you

- **Secrets:** metadata provider keys and the parental PIN are never stored in this repository. Keys are kept in the operating system's credential store, and the PIN and API keys are stored only as salted hashes.
- **Media server:** remote clients must sign in or present an API key, router ports and any relay open only when you start remote connectivity, and turning off the Remote Access switch refuses remote clients outright.
- **Playback:** the built-in player streams over loopback with a per-session token.
- **Supply chain:** dependencies are locked, CodeQL runs on pull requests where configured, and dependencies are audited on a schedule.

## What this repo is

Cinavault-3.0: Windows-native CinaVault server and desktop-client foundation. It is not the canonical CinaVault-Premium product; see johngraven75/CinaVault-Premium for the current flagship release.
