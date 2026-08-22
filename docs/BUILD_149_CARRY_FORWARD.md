# Build 149 Carry Forward Checklist

This file records the required release checks for Build 149 and later.

## Required checks

- The app starts successfully.
- All main navigation areas remain available.
- Existing media, source, download, live TV, server, security, remote access, advanced, cloud, plugin, AI, and settings areas remain reachable.
- Media scanning finishes and reports a summary.
- Metadata posting finishes and reports a summary.
- AI configuration loads before inference.
- Stored configuration is reused after restart.
- Poster card sizing remains stable.
- Installer/package workflow remains valid.
- Release notes identify preserved features, repaired regressions, known limits, and manual checks.

## Rule

Do not remove or hide an accepted feature from a prior build unless the owner explicitly approves the change and the release notes document it.
