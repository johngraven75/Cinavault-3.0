# Total Automation Policy

This repository follows the owner's repo-wide automation rules.

- Preserve accepted functionality; do not remove features merely to make CI pass.
- Inspect failures, fix root causes, commit, and rerun until green or until a hard external blocker is proven.
- Use clean restore/build/test/package validation appropriate to the repository stack.
- Treat compile success as necessary but not sufficient: verify runtime/UI-critical paths where applicable.
- Retain useful build artifacts and diagnostics.
- Keep settings, migrations, and persistent state compatible across upgrades.
- Prefer automated build, test, packaging, release, cleanup, and maintenance workflows.
- Never report a build/release as green without current evidence for the current commit.
- Keep repository structure organized and generated outputs controlled.
- Do not commit secrets, signing keys, tokens, credentials, or private certificates.
