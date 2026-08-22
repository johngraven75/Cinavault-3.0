# Repository Engineering Standard

Mandatory for all AI-assisted engineering. Before code: define product goal, target user, measurable success criteria, non-goals, assumptions, and missing requirements. Propose architecture first, including security, lifecycle, persistence, public contracts, observability, backward compatibility, upgrades and rollback.

Organize implementation/reporting as **Frontend**, **Connector / integration**, and **Backend** where applicable. Break work into independently testable vertical slices suitable for isolated git worktrees/branches. For each slice specify exact files, public interfaces/contracts, validation/error handling, logging/telemetry, security implications, and definition of done. Keep commits atomic and conventional; never merge around failing required checks.

Ship fresh, idiomatic, repository-specific production code following SOLID, DRY, explicit typing, immutable data where practical, dependency injection, narrow interfaces, secure defaults, input validation, null safety, cancellation/disposal, bounded resources, and clear separation of concerns. No TODOs, placeholders, mock data, fabricated integrations, credentials, or incomplete production paths. Preserve accepted functionality and backward compatibility unless an explicit migration is approved.

Every slice requires appropriate unit tests for edge/failure cases, real filesystem/OS integration tests, API/IPC contract tests, security tests for injection/traversal/unsafe deserialization/privilege escalation, performance tests with measurable budgets, and manual QA for UI/installer flows. Run formatting, linting, static analysis, type checks, unit/integration/E2E, packaging, and user-flow validation. Never weaken tests to obtain green CI.

Before completion, self-review normal and failure paths for races, deadlocks, leaks, disposal, file locks, cancellation, retries, process cleanup, privilege boundaries, interrupted operations, rollback, and compatibility; fix discovered issues first.

Production readiness requires a diff summary, changelog/release notes, migration/installer notes, public API docs, environment/config docs, rollback plan, post-release monitoring plan, and test/checksum evidence for packaged artifacts. Never claim completion/publication/signing/test success without evidence. Commit every change before building and rerun validation after each fix.

## Windows desktop requirements
This is a premium Windows desktop application. Preserve the established desktop framework unless migration is explicitly in scope; apply WinUI 3/WPF best practices where applicable. Explicitly handle Windows long paths/path normalization, UAC and least privilege, registry ownership/cleanup, file locks, process/service lifecycle, MSBuild/toolchain behavior, threading/UI affinity, crash reporting/recovery, and backward compatibility. Maintain MSIX/MSI compatibility where supported, silent install/uninstall, safe auto-updates, resilient background processes/services, correct per-user/per-machine semantics, and upgrade/rollback safety. The code must build with `build.cmd` when present and pass all existing tests in each isolated worktree before merge/release.
