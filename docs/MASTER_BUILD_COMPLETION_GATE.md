# CinaVault Master Build Completion Gate

This gate applies to every build, repair, feature change, platform migration, package, and release in CinaVault Premium, CinaVault Android, and CinaVault iOS. A change is not complete merely because it compiles.

## Required workflow

Every change must proceed through:

1. Discovery and repository understanding
2. Root-cause diagnosis
3. Complete-file implementation
4. Dependency and configuration validation
5. Static code review
6. Clean build
7. Automated tests
8. Runtime and UI exercise
9. Persistence and migration validation
10. Network/API failure-path validation where applicable
11. Security and privacy review
12. Performance and freeze/hang review
13. Cross-platform parity review
14. Packaging validation
15. Installation and launch validation where the environment permits
16. Final regression review

## Mandatory status vocabulary

Only these evidence states are permitted:

- `verified`
- `fixed_and_verified`
- `fixed_not_executable_in_environment`
- `external_blocker`
- `in_progress`
- `not_applicable`

`not_applicable` requires a concrete architectural reason. `external_blocker` requires the exact cause, affected platform, and resolution requirement.

## Release rule

A release workflow must run the master gate in release mode. Release mode fails unless:

- the overall build status is `verified`;
- release authorization is true;
- every required phase is verified or has a documented external blocker;
- every build-specific acceptance criterion is verified or has a documented external blocker;
- no unresolved critical or high-severity defect remains;
- changed workflows were executed successfully;
- packaging artifacts were verified;
- success is supported by recorded evidence rather than expectation.

## Cross-platform rule

Windows Premium is the functional and visual reference unless a platform-specific implementation is required. Android and iOS must receive the same user-facing function, option, design intent, security behavior, defect repair, and regression protection.

## Persistent-state rule

Upgrades must preserve user data, settings, provider configuration, credentials, custom endpoints, library state, and source definitions. New defaults must be merged safely. No build may silently disable providers or erase established configuration.

## Current metadata/poster release requirements

The metadata/poster repair release cannot be authorized until the evidence record proves:

- a current media title is matched through a working provider;
- metadata fields are returned and written to SQLite;
- poster information is returned;
- poster bytes are downloaded or securely proxied;
- artwork is validated as an image;
- artwork is cached or served through the approved secure path;
- the correct poster renders on the corresponding library card;
- the visible library total equals the actual indexed media-file count;
- provider enablement persists across restarts, upgrades, and supported operating systems;
- MSI and NSIS installers are built only after the functional gate passes;
- installer artifacts are non-empty and checksums are verified.

The machine-readable evidence file is `build-verification/current-build.json`. The executable validator is `scripts/verify-master-build-gate.mjs`.
