# Build 149 Recent Build Audit

This audit records recent build work that must be preserved while repairing Build 149.

## Recent build items to preserve

| Build | Area | Preserve |
|---|---|---|
| 149 | Card sizing | Media cards and poster cards must stay stable. |
| 149 | Security and packaging | Security updates and installer workflow must remain intact. |
| 148 | Startup helpers | Startup media helper plugins must remain enabled. |
| 147 | Media rows | Clean media rows and poster sizing must remain fixed. |
| 145 | AI fallback | AI workflows must fail safely and keep diagnostics visible. |
| 145 | Feature suite | The full media server feature suite must remain reachable. |
| 144 | Server foundation | CinaVault server foundation must remain preserved. |
| 143 | Card clamps | Oversized media cards must stay clamped. |
| 143 | AI media agent | AI media agent access must remain available. |
| 142 | Plugin safety | Plugin safety behavior must remain valid. |
| 141 | Casting and installers | Cast support and clean installers must remain valid. |
| 140 | Packaging | Installer artifacts and lockfile consistency must remain valid. |

## Build 149 risks addressed in this branch

- The AI Diagnostics screen now loads current configuration when opened.
- The AI Diagnostics screen now uses the backend default model by default.
- Metadata posting now uses a bounded batch and skips already-complete items.
- Visible provider messages are capped so the activity log stays usable.
- The media-center sidebar preserves access to every existing feature area.
- The new app shell keeps the original tab/component map intact.
- A Build 149 carry-forward checklist has been added.

## Remaining required checks

- Review backend AI lookup compatibility.
- Review source scanning for duplicate path and recursion safeguards.
- Review metadata backend summaries for no-provider and no-match cases.
- Run installer/package workflow before final release.
- Keep this audit visible until all Build 149 acceptance checks pass.
