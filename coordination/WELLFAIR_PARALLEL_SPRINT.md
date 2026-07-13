# WellFair Parallel Sprint — Sub-agent orchestration (2026-07-02)

**Canonical repo:** `C:\Projects\qualia-27062026` | **Branch:** `0.0.24`  
**Goal:** Close Phase 1 P2 gaps + Phase 2 Q2 remainder in one coordinated sprint.

## Orchestrator (Grok)

- Assigns non-overlapping file ownership per lane
- Reviews each lane diff + runs `cargo test -p qualia-client-core wellfair --lib`
- Integrates, fixes collisions, commits single stack

## Lanes (parallel, no shared file edits)

| Lane | Agent | Scope | Owns | Gate |
|------|-------|-------|------|------|
| **A** | WS1-outbox | Durable signed sync outbox hook on vault commit | `sync_outbox.rs`, `vault.rs` (append hook only), `api.rs` (enqueue), `mod.rs` | Outbox round-trip + idempotent enqueue tests |
| **B** | WS1-replay | Crash/replay idempotency at WAL checkpoint boundary | `replay_tests.rs`, `vault.rs` (read-only helpers if needed) | Reopen vault → graph/journal counts match |
| **C** | Q2-conditions | Conditions/allergies self-report records + Personal UI | `wellfare-core/conditions.rs`, `personal_profile.rs`, `personal_panel.rs`, `api.rs`, Tauri commands | Record compiles to journal kind `condition` |

## Explicitly deferred (next sprint)

- §8.1 full E2E automation (needs A+B stable)
- OS med reminders, external drug lookup
- Sanctuary / Phase 3

## Review checklist (orchestrator)

1. No second authority (all writes via `WebizenHostApi` / `VaultService`)
2. Policy fail-closed on classified writes
3. Tests pass: `cargo test -p qualia-client-core wellfair --lib`
4. `cargo check -p webizen-studio` + `webizen-desktop` if UI/commands touched
5. NOTICES.md RELEASE line after merge

## Verification command

```powershell
cd C:\Projects\qualia-27062026
cargo test -p qualia-client-core wellfair --lib
cargo check -p webizen-studio -p webizen-desktop
```