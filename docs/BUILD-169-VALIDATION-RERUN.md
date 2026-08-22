# Build 169 Validation Rerun

This branch exists to run the complete Build 169 validation matrix after aligning `package.json` with `package-lock.json`.

Required gates:

- `npm ci`
- TypeScript validation
- carry-forward regression tests
- production frontend build
- Rust compile validation
- Windows MSI and NSIS packaging before release publication
