# Repository Engineering Rules

## Forward-thinking implementation standard

All human and AI-assisted codework in this repository must be forward-thinking, innovative, effective, and precise.

### Purpose and architecture

- Begin every implementation with a clear purpose, intended user outcome, constraints, and measurable completion criteria.
- Organize implementation and reporting into explicit **Front end**, **Connector / integration**, and **Back end** sections when those layers apply.
- Define contracts between layers before implementation. Keep presentation, transport/integration, and domain logic separated.
- If a layer does not apply, state that explicitly instead of adding unnecessary architecture.

### Implementation quality

- Design for maintainability, security, performance, accessibility, observability, and future upgrades.
- Prefer cohesive root-cause solutions over piecemeal patches.
- Innovation must provide practical value and must not introduce avoidable complexity or regressions.
- Use repository-specific, freshly reasoned code. Do not submit generic “big-box” code walls, placeholders, fabricated integrations, or unreviewed boilerplate.
- Preserve accepted functionality unless removal is explicitly approved and documented.

### Validation and publication gate

- Define the validation plan before publication.
- Run the relevant formatting, static analysis, type checks, unit tests, integration tests, packaging checks, and user-flow tests supported by the repository.
- Test cross-layer behavior whenever front end, connector, and back end interact.
- Do not publish, release, merge, or describe work as complete while required checks fail.
- If full validation is blocked, identify the exact blocker and leave the work explicitly incomplete; do not imply success.
- Never weaken security controls or remove meaningful tests merely to obtain a green build.

### Required completion report

Every completed code task must report:

1. **Purpose** — the problem solved and intended outcome.
2. **Front end** — user-visible changes and validation, or “not applicable.”
3. **Connector / integration** — APIs, IPC, storage, provider, or platform wiring and validation, or “not applicable.”
4. **Back end** — domain logic, services, persistence, and validation, or “not applicable.”
5. **Completion** — files/components changed, tests run, results, remaining risks, and publication/release status.
