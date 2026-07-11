# Webizen Desktop Human Product Overhaul Plan

Last updated: 2026-07-11

## Why this plan exists

The current Webizen desktop app still behaves like a technical diagnostic shell. It can expose useful subsystems, but the main experience does not yet answer the human user's first question: "What can I safely do here, right now?" It also still has freeze risks: blocking file and update work can be reached from UI/server paths, some locks and panics can poison long-lived state, and several background loops are not managed by a clear supervisor with cancellation, progress, or failure reporting.

This plan treats responsiveness, useful observability, and human-facing workflows as release criteria. Diagnostics stay available, but they move out of the primary user path.

## Non-negotiable product criteria

1. The UI must stay interactive while GPU, MCP, update, import, daemon, logging, and file operations run.
2. Any operation expected to take more than 150 ms must expose progress, cancellation, and a user-readable status.
3. Panics and poisoned locks must not freeze the visible shell. They must become logged problem reports and recoverable task failures wherever possible.
4. The default home screen must be for end users. Developer diagnostics must live behind a Developer Tools mode.
5. Logs must explain what happened in human and engineering terms: session id, operation id, command name, duration, error code, thread/task class, and the last user-visible action.
6. No fake or stubbed functionality in the main UI. A visible action either calls a real backend path or is explicitly marked unavailable with the reason.
7. The installer and updater must remain first-class. Update checks must not block startup or menu interaction.

## Current Failure Findings

These are the current hotspots that need direct implementation work:

- `crates/webizen-desktop/src/main.rs` still has `expect`, `unwrap`, blocking static-protocol file reads, tray icon failure panic risk, updater install calls, and long-running loops.
- `crates/webizen-desktop/src/settings_server.rs` still performs synchronous `std::fs` work and `Mutex::lock().unwrap()` inside HTTP handlers.
- `crates/webizen-desktop/src/runtime.rs` creates a WGPU backend with `pollster::block_on` and owns a background loop that needs explicit supervision and crash reporting.
- `crates/webizen-desktop/src/native_surface.rs` runs a render loop with many mutex reads/writes and no explicit cancellation/reporting contract.
- `crates/webizen-desktop/src/commands/mod.rs` mixes many user-invoked commands, blocking filesystem work, heavy CPU/GPU work, and partial `spawn_blocking` coverage.
- `crates/webizen-desktop/src/mcp_server.rs` has panic-prone socket clone handling.
- `crates/webizen-studio/src/main.rs` and component modules still have polling loops and diagnostics-heavy routes that dominate the product surface.
- The current logs page can show lines, but it does not yet answer "what operation hung, what was the user doing, and what should they do next?"

## Target Architecture

### 1. Desktop Supervisor

Add a `DesktopSupervisor` service in `crates/webizen-desktop/src/supervisor.rs`.

Responsibilities:

- Register every long-lived service: settings server, runtime engine, native renderer, MCP server, updater, telemetry poller, reminder notifier, and host API bridge.
- Own cancellation tokens for every loop.
- Emit structured `TaskEvent` records: `Started`, `Progress`, `Heartbeat`, `Warning`, `Failed`, `Cancelled`, `Completed`.
- Track task status in memory for UI display and problem reports.
- Restart recoverable background tasks with bounded backoff.
- Shut down cleanly on app exit.

Every existing forever loop should be migrated into a supervisor-managed task. No unmanaged `loop { sleep(...) }` should remain in desktop runtime code.

### 2. Work Pools

Separate work by class:

- UI/main thread: window creation, menu dispatch, light state changes only.
- Async IO pool: HTTP handlers, config reads, small network calls, updater checks.
- Blocking IO pool: large file import/export, bundle assembly, log compression.
- CPU pool: parsing, model/index analysis, report generation.
- GPU/render thread: WGPU initialization and frame rendering only, controlled by messages.

The UI and HTTP handlers should communicate with services through channels and task ids, not by holding locks across operations.

### 3. Panic and Lock Boundaries

Implement a command boundary wrapper for Tauri commands and menu-triggered jobs:

- Capture panic payloads with `catch_unwind` at job boundaries.
- Convert poisoned locks into typed errors with context.
- Log a problem event with operation id and user action.
- Return a user-readable failure instead of freezing or silently failing.

Replace runtime-path `unwrap` and `expect` with typed errors or logged fallbacks. Test-only unwraps can remain in test modules.

### 4. Non-Blocking Filesystem and Updater

Move all filesystem work in HTTP handlers and Tauri commands to `tokio::fs` or `spawn_blocking`, depending on whether the work is async-friendly or CPU-heavy.

Updater behavior:

- Startup checks run in the background after the first window is usable.
- Manual "Check for Updates" creates a supervised job with progress.
- Download/install preparation must show progress and cancellation where Tauri allows it.
- The app must not call `download_and_install` from a menu or tray callback without first creating a supervised job and visible status.

### 5. Useful Observability

Replace the current line-oriented diagnostic log with a layered logging system:

- `tracing` spans for every command, menu action, HTTP route, background task, and updater action.
- Rolling JSON log for machine-readable diagnostics.
- Rolling human log for quick inspection.
- In-memory recent events for UI.
- Session marker on startup and shutdown.
- Hang watchdog recording UI heartbeat, active tasks, last route, last menu command, and last backend command.
- "Export Problem Report" action that creates a redacted zip with logs, config summary, versions, GPU/WebView2 details, route history, task state, and crash markers.

The logs page should become a Problems page first, raw logs second.

## Human UX Direction

The desktop app should open into a human product, not a diagnostic index.

### Primary Navigation

Default nav:

- Home
- WellFair
- Library
- Browser
- QApps
- Sanctuary
- Work
- Updates

Secondary nav:

- Settings
- Problems
- Developer Tools

Developer Tools should contain MCP status, raw command harnesses, render probes, hardware diagnostics, and low-level logs.

### Home Screen

The first screen should show:

- Current vault/app status: local data path, locked/unlocked state, sync/offline state, update status.
- Continue where you left off: recent WellFair record, recent QApp, recent import, recent document.
- Clear primary actions: import health data, open Library, start local Browser, install/update QApps.
- Problems requiring attention: crashed background task, missing daemon, failed update, unavailable GPU, import failure.

It should not lead with MCP, raw telemetry, route lists, or debug cards.

### WellFair MVP

Make at least one end-user workflow complete and real:

1. Import Samsung Health CSV or supported local health export.
2. Parse and store locally.
3. Show timeline, sleep summary, activity summary, medication/reminder status, and source provenance.
4. Export/share a user-selected report.
5. Surface privacy state and local-only guarantees.

No clinical claims should be presented as diagnosis. Health copy should remain careful and explanatory.

### QApps and Browser

QApps should feel like installable tools, not internal route demos:

- Catalog/library view.
- Installed/running status.
- Open, update, remove, inspect permissions.
- Per-QApp error state and logs.

Browser should be a local app browser with stable navigation, history, reload, stop, open externally, and visible loading/error states.

## Implementation Phases

### Phase 0: Stabilization Harness

Deliverables:

- Add a desktop smoke-test checklist and script for `/`, `/design-studio.html`, `/logs`, `/api/status`, `/api/logs`, Studio routes, menu actions, and updater feed.
- Add a controlled panic command and controlled slow command in Developer Tools only.
- Verify that panic and slow command tests do not freeze the shell.
- Capture before/after hang reports.

Acceptance:

- A panic test creates a problem report and keeps the app usable.
- A slow test shows progress and cancellation.
- Route tests pass locally before release.

### Phase 1: Supervisor and Logging Foundation

Deliverables:

- Implement `DesktopSupervisor`, `TaskId`, `TaskEvent`, `TaskSnapshot`, and `ProblemReport`.
- Convert settings server, telemetry loop, runtime loop, native render loop, update check, and reminder notifier into supervised tasks.
- Add `tracing` subscriber with rolling JSON and human logs.
- Add UI heartbeat endpoint and backend watchdog.
- Add tray/menu controls for log level, Problems, Export Problem Report, and Developer Mode.

Acceptance:

- No desktop background loop runs without cancellation.
- Active tasks are visible in `/api/status`.
- Logs include operation ids and task ids.

### Phase 2: Remove Blocking Paths

Deliverables:

- Replace blocking `std::fs` in HTTP handlers with async or blocking-pool jobs.
- Move large command filesystem work behind supervised jobs.
- Replace runtime-path `unwrap`/`expect` in desktop code with typed errors or fallback handling.
- Convert update download/install to a supervised job.
- Audit `Mutex` usage and remove lock holds across async or heavy work.

Acceptance:

- Menu actions and route loads remain responsive during imports, update checks, render initialization, and daemon failure.
- `rg "unwrap\\(|expect\\(|std::fs::|thread::sleep"` findings in desktop runtime paths are either removed, justified in tests, or tracked in a remediation list.

### Phase 3: Human Shell Redesign

Deliverables:

- Replace diagnostic-first home with user dashboard.
- Move raw diagnostics into Developer Tools.
- Make menu actions match visible UI state.
- Add clear empty/error/loading states for Home, WellFair, Library, Browser, QApps, Updates, and Problems.
- Add first-run flow for data folder, privacy mode, developer mode, and update preference.

Acceptance:

- A non-technical user can understand what to do from the first screen.
- Diagnostic pages are not the primary experience.
- Every visible primary button performs a real action or explains why it is unavailable.

### Phase 4: Real User Workflows

Deliverables:

- Complete WellFair import, review, local storage, report export, and reminder path.
- Complete QApp catalog/install/open/update/remove flow.
- Complete Browser navigation controls and failure handling.
- Complete Library recent files/imports/provenance view.

Acceptance:

- At least one complete WellFair workflow can be tested from fresh install to export.
- QApps can be managed without using MCP or raw diagnostic tools.
- Browser errors are visible and recoverable.

### Phase 5: Release Gates

Deliverables:

- Add CI jobs for desktop smoke checks, updater manifest validation, route availability, and packaging.
- Add manual release checklist for Windows installer, Windows updater, macOS Apple Silicon bundle, and update feed.
- Add hang regression checklist: slow import, failed daemon, failed update, GPU unavailable, MCP unavailable, controlled panic.

Acceptance:

- A release cannot pass if the app starts into a broken route, the update feed is invalid, or the controlled panic freezes the UI.
- Release notes include user-facing changes, known issues, and how to export problem reports.

## Immediate Engineering Task List

1. Create `supervisor.rs` and route all background loops through it.
2. Introduce structured `tracing` and replace the current ad hoc log writer with a sink that feeds files, memory, and Problems UI.
3. Add UI heartbeat and backend watchdog.
4. Convert updater actions into supervised jobs.
5. Convert settings server filesystem handlers to non-blocking implementations.
6. Wrap command/menu execution with panic and duration boundaries.
7. Redesign Studio Home into a user dashboard and move diagnostics to Developer Tools.
8. Complete one real WellFair workflow end to end.

## What Must Stop Happening

- No more visible buttons that only exercise internal diagnostics.
- No more silent hangs where the user cannot tell what failed.
- No more raw logs as the only support tool.
- No more long-running work launched directly from menu/tray callbacks.
- No more main user path dominated by MCP, render probes, hardware telemetry, or debug panels.

## Definition of Done

This overhaul is done when a fresh installed user can:

1. Open the app and see a clear, useful home screen.
2. Import or open real local content.
3. Use at least one complete WellFair/QApp/Browser workflow.
4. Check for updates without freezing the app.
5. Recover from a failed subsystem without restarting.
6. Export a problem report that explains the failure.
7. Keep using the app while slow work runs in the background.

