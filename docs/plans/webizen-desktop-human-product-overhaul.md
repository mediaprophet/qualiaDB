# Webizen Desktop Human Product Overhaul

Status: active implementation plan; stabilization tranche completed

Last reviewed: 2026-07-11

Scope: `webizen-desktop`, `webizen-studio`, the local settings/companion server, native browser windows, GPU surfaces, installer, and updater

## 0. Implementation Ledger (2026-07-11)

Completed in `0.0.24`:

- Split the loopback settings/control API from the paired LAN companion WebSocket. The control API now binds only to `127.0.0.1`, and CORS accepts only Tauri and same-port loopback origins.
- Removed duplicate native menu dispatch, removed false tab actions, corrected backup/update/GPU routes, and added an automated route-table test for every native destination.
- Moved Samsung import, relay sync, backup export, backup restore, and diagnostics onto blocking workers instead of the UI/event-loop path.
- Added a bounded asynchronous rotating desktop logger, session/thread context, panic crash markers, readable local logs, and tray access.
- Added a desktop supervisor with explicit service and operation lifecycle snapshots. Settings status now reports runtime, host API, settings API, companion gateway, and managed operations.
- Stopped silent startup update installation. Menu and tray checks now share one implementation and show real user decisions and results.
- Replaced the diagnostic-first native dashboard and synthetic telemetry with a human home for health, library, work, identity, creative tools, backup/sync, and Sanctuary.
- Replaced the broken Design Studio WASM/docs bootstrap with the working local design graph, canvas, SPARQL enrichment, resource recommendation, queue, and saved-design implementation.
- Added refresh-safe Studio routes for primary destinations and a real `/dashboard` Dioxus route.
- Fixed `scripts/build_frontend.ps1` so the Dioxus 0.7 output is actually staged into `webizen-studio/dist`; stale hashed JS/WASM files are removed before packaging.
- Built and smoke-tested the Windows release executable locally. The desktop library suite passes with 26 tests passed and two adapter-dependent GPU tests ignored.

Still required for the complete product contract:

- Replace the remaining generic `/api/invoke/{cmd}` loopback bridge with narrow typed portal endpoints and capability checks.
- Move all remaining long-running commands and background loops under supervisor-owned cancellation and shutdown, not only the highest-risk WellFair operations.
- Replace the remaining static capability/status labels in individual QApps and secondary screens with measured state or hide them.
- Complete the persistent updater service, signed previous-version update tests, rollback/recovery UX, and installer/updater release gates on Windows and Apple Silicon.
- Add Windows UI automation for every enabled menu item, long-operation responsiveness stress tests, and real GPU/native-child-window teardown tests.
- Finish the Problems and Operations UI so every supervised failure has a human recovery action, not only logs and status snapshots.

## 1. Outcome

Webizen Desktop must become a dependable local application for a human owner. The main window must remain responsive, menus must perform the action they name, long work must be visible and cancellable, and failures must become recoverable problems rather than frozen windows.

The product is not complete while its main surfaces expose diagnostics, route demos, synthetic records, placeholder panels, or generic Studio editors in place of working user flows. Developer and MCP tools remain valuable, but they belong in an explicitly enabled Developer Tools area.

This plan is based on a code review, not only the observed symptoms. It replaces the earlier broad roadmap with concrete defects, ownership boundaries, implementation packages, and release gates.

## 2. Non-Negotiable Product Contract

1. The main WebView and native event loop remain interactive during startup, import, backup, sync, update, rendering, inference, and daemon failure.
2. Work expected to exceed 150 ms runs as a managed operation with an operation id, state, progress when measurable, cancellation when safe, and a final result.
3. A panic in a worker or renderer cannot silently stop the shell. A panic on the event-loop thread must produce a crash marker that is shown on the next launch.
4. No primary action is a stub. A visible action either completes a real workflow or is hidden until its backend and recovery path are implemented.
5. Product status is truthful. `Active`, `Installed`, `Beta`, `Ready`, and `Online` are derived from runtime capability checks, not static catalog labels.
6. The default experience is useful without MCP, raw logs, a route list, or a command harness.
7. Local data and native commands are not reachable from an unauthenticated LAN or arbitrary web origin.
8. Updates are signed, user-visible, resumable where supported, and never silently installed during startup.
9. Windows x64 and macOS Apple Silicon are equal release targets. Platform-specific functionality must degrade clearly when it is not available.

## 3. Current Architecture

The intended desktop architecture is viable, but its boundaries are currently blurred:

- Tauri owns the native process, main OS event loop, menus, tray, updater, windows, and native command bridge.
- The main window loads the bundled `webizen-studio` Dioxus WASM application in the Tauri WebView. This is the application shell, not an iframe.
- The Browser page still renders `qualia://` and `webizen://` content in iframes. External HTTP(S) URLs are redirected to a separate top-level Tauri WebView window.
- Windows native GPU content uses a child HWND and a dedicated render thread. Other visual surfaces use the main WebView or off-screen rendering.
- An Axum server currently combines loopback settings, static portal assets, Studio diagnostics, jobs, SPARQL proxying, command invocation, and LAN companion WebSocket routes on one port.
- The local daemon, WGPU runtime, persistence worker, reminder poller, render preview loop, telemetry loops, settings server, MCP server, host API, and updater all have independent lifecycle models.

The target is still Tauri plus Dioxus WASM plus purpose-built native WebViews. The overhaul does not replace the working stack. It gives each part one owner and removes duplicate control paths.

## 4. Reviewed Findings

Severity meanings:

- Critical: security exposure or credible data-loss/control risk.
- P0: directly explains hangs, broken menus, or unusable primary workflows.
- P1: high-impact reliability, supportability, or product-integrity issue.
- P2: important cleanup or release hardening after the main path works.

### 4.1 Critical Security Finding

#### SEC-01: LAN clients and arbitrary origins can reach the native invoke bridge

Evidence:

- `settings_server.rs` binds the combined server to `0.0.0.0`.
- The router applies `CorsLayer::permissive()`.
- `POST /api/invoke/{cmd}` accepts an arbitrary command name and dispatches it through the main Tauri WebView invoke handler.
- The desktop invoke registry contains hundreds of commands, including filesystem, vault, import, export, sync, rendering, and execution-adjacent operations.

Impact:

Any device able to reach the port, or a browser page allowed by permissive CORS, may attempt native command invocation without an authenticated, capability-scoped session. This is also a reliability problem because remote requests can enqueue blocking work through the same dispatch path as the UI.

Required correction:

- Split the loopback control plane from the LAN companion gateway.
- Bind settings, logs, jobs, and command APIs to `127.0.0.1` and `::1` only.
- Remove the generic `/api/invoke/{cmd}` route. Replace it with typed endpoints for the small set of portal operations that are actually required.
- Give the companion gateway its own listener, authenticated pairing session, origin policy, message schema, rate limits, body limits, and narrow capability set.
- Use exact CORS origins or no browser CORS at all. Reject missing/invalid session tokens before parsing large bodies or acquiring host locks.
- Add a test proving a LAN socket cannot reach loopback APIs and an unpaired client cannot mutate companion state.

### 4.2 P0 Responsiveness and Correctness Findings

#### REL-01: Native menu actions have duplicate, inconsistent, and no-op paths

Evidence:

- `navigate_main_to` both injects `history.pushState` and emits `shell-navigate`; Studio responds to the event by navigating again.
- Back, forward, and reload both evaluate JavaScript directly and emit events whose listeners repeat the action.
- `toggle_gpu` requests `gpu-viewport`, but the native route map has no `gpu-viewport` entry and falls back to `/`.
- File > New Tab returns to the dashboard rather than creating a tab.
- File > Close Tab also returns to the dashboard.
- The Studio listener says New Window and GPU toggle are not implemented even though native menu code presents those actions.
- `backup` emits `shell-backup`, while Studio listens for `open-backup`.
- Samsung import navigates to Tools rather than opening and completing the import flow.
- Menu and tray update implementations are separate and behave differently.

Impact:

This directly matches the report that File, View, QApps, Tools, and Help appear not to work. Some actions happen twice, some navigate to the wrong route, and some only write a console message.

Required correction:

- Introduce one typed `ShellAction` enum shared by menu, tray, keyboard shortcuts, and Studio.
- Dispatch each action exactly once through a `ShellController`.
- Return an acknowledgement containing action id, result, and active route/window.
- Derive menu enabled/checked state from shell state.
- Remove New Tab until the product has real tab ownership, or implement tabs through the Browser controller. Do not relabel dashboard navigation as tab creation.
- Add an automated table test covering every menu id and a Windows UI smoke test that invokes every enabled item.

#### REL-02: Synchronous commands perform heavy work while holding the global host API mutex

Evidence:

- Samsung Health folder import is a synchronous Tauri command and holds `HostApiState` across the complete import.
- Relay sync, backup export, backup restore, diagnostics, and other WellFair commands also hold the host mutex while doing network, filesystem, parsing, or database work.
- Most registered Tauri commands are synchronous functions; only a minority explicitly use async or `spawn_blocking` boundaries.

Impact:

One slow operation serializes unrelated host actions. Depending on Tauri dispatch and caller context, synchronous work can also block IPC processing or the event loop. A poisoned lock can make all subsequent human workflows fail.

Required correction:

- Replace coarse `Arc<Mutex<Option<WebizenHostApi>>>` ownership with a `HostService` actor or command queue.
- Clone immutable request inputs before scheduling work. Never hold a shared state lock across filesystem, network, database, GPU, or parsing work.
- Classify every exposed command as `UiImmediate`, `AsyncIo`, `BlockingIo`, `Cpu`, `Gpu`, or `ServiceActor`.
- Convert long commands to managed jobs and return an operation id immediately.
- Add contention tests: run import, status, cancel, menu navigation, and log export concurrently and assert bounded response times.

#### REL-03: Startup still performs blocking and failure-prone work in Tauri setup

Evidence:

- Tauri `setup` scans daemon ports with a synchronous bind loop.
- `start_qualia_protocol()` is called synchronously in setup.
- Tray icon decoding uses `expect`.
- App-state config uses `lock().unwrap()` before launch.
- Startup immediately launches many services and an ontology seed without a single readiness model.

Impact:

The first window can be visible while startup still monopolizes or destabilizes native setup. Failures are split between `eprintln`, the ad hoc log, events, and silent return paths.

Required correction:

- Keep setup limited to state registration, window/menu creation, and supervisor start.
- Move port selection, protocol startup, host API opening, seeding, daemon startup, and GPU initialization into supervised operations after first paint.
- Establish explicit `Starting`, `Ready`, `Degraded`, `Failed`, and `Stopped` service states.
- Show a usable shell immediately; unavailable features display their service state without blocking other routes.

#### REL-04: Custom protocol handlers do synchronous I/O and host work

Evidence:

- `qualia://` reads files synchronously per request.
- render and anatomy responses clone complete image buffers behind mutexes.
- anatomy JSON and `.10d` requests hold the global host API lock while performing host calls and loading cached assets.

Impact:

WebView resource requests can contend with user commands and large buffers. A slow disk, large asset, or poisoned lock can make a page appear frozen.

Required correction:

- Serve immutable QApp assets through a bounded asset service with canonical path validation, size limits, cache headers, and pre-opened/cached metadata.
- Store render frames as immutable `Arc<[u8]>` snapshots or a bounded swap buffer so protocol reads do not clone under a producer lock.
- Route anatomy asset loading through the host service and cache completed immutable responses.
- Return typed 4xx/5xx responses with correlation ids and log the duration.

#### REL-05: Long-lived tasks are unmanaged and cannot shut down cleanly

Evidence:

- Reminder, telemetry, render-preview, ambient telemetry, runtime kernel, persistence, native render, settings, MCP, daemon, and Studio polling loops all use separate loops or threads.
- Several loops have no cancellation token, join handle, heartbeat, or bounded restart policy.
- Studio component effects start infinite polling futures and retain Tauri callbacks with `Closure::forget()` without keeping unlisten handles.

Impact:

Tasks can outlive the route or window that created them, duplicate after remount, fail silently, or keep stale references. Shutdown and restart behavior is not deterministic.

Required correction:

- Add `DesktopSupervisor` and make it the sole owner of long-lived native tasks.
- Add a Studio `AppRuntime` context that owns polling subscriptions and Tauri unlisten functions for the lifetime of the root app.
- Use cancellation tokens, join handles, heartbeat deadlines, and bounded restart policies.
- Stop route-level infinite polling. Use a central event stream with backoff and visibility-aware refresh.

#### REL-06: Native renderer state is panic-prone and overly lock-coupled

Evidence:

- The render loop repeatedly calls `Mutex::lock().unwrap()` for renderer, camera, time, dimensions, scene, and child HWND state.
- Mount, camera, mesh upload, asset load, and unmount commands mutate the same lock graph from other threads.
- The render thread has no panic boundary, task state, or failure event.

Impact:

A single poisoned lock terminates the render thread. Lock ordering and renderer mutation can stall commands or produce an invisible dead renderer while the UI continues to request frames.

Required correction:

- Make the render thread the sole owner of renderer and surface resources.
- Send typed render commands through a bounded channel and publish immutable status/frame snapshots.
- Coalesce camera and resize updates instead of queueing every input event.
- Catch render-thread panics at the thread boundary, mark the service failed, release the child surface, and offer a software/WebView fallback.
- Implement equivalent Metal initialization and capability reporting for Apple Silicon; do not present Windows HWND controls on macOS.

#### REL-07: Updater behavior is duplicated, silent, and unsafe for startup UX

Evidence:

- Startup checks and immediately calls `download_and_install` when an update exists.
- Tray update, native Help menu update, and startup update use separate implementations.
- Startup and tray paths discard progress and errors to `eprintln` or ignore the result.
- There is no shared update state, cancellation, release-note view, restart flow, or recovery state.

Impact:

The app can download and install while the user is trying to start it, with no useful status. Multiple checks can race. An installer failure is difficult to diagnose.

Required correction:

- Add one `UpdateService` owned by the supervisor.
- Startup performs a delayed background check only after first paint and never installs without the configured user policy.
- Show `Idle`, `Checking`, `Available`, `Downloading`, `Verifying`, `ReadyToRestart`, and `Failed` states in Updates and tray/menu state.
- Stream byte progress, verify signature and platform, preserve the current installer/updater artifacts, and provide explicit Restart Now/Later actions.
- Persist update failure details and recover cleanly on next launch.

#### REL-08: Logging performs synchronous disk I/O on every call and lacks operation context

Evidence:

- `desktop_log::record` locks the in-memory queue, creates directories, opens the log file, appends one line, and closes it for every event.
- The panic hook writes through the same logging path.
- Many important paths still use `println` or `eprintln`, which disappear in a packaged Windows GUI app.
- Entries contain only timestamp, level, and free-form message.

Impact:

Logging can add I/O latency to menu, updater, and startup paths. It cannot identify the active operation, last completed stage, service state, or reason for a hang.

Required correction:

- Use `tracing` spans and a dedicated non-blocking writer with bounded buffering and rotation.
- Record session id, operation id, parent operation, service, action, stage, duration, thread/task, error code, and user-safe message.
- Maintain structured JSON logs and a concise human log.
- The panic hook must write a minimal crash marker through a panic-safe path, then let the normal reporter enrich it on next launch.
- Add a watchdog snapshot containing UI heartbeat age, event-loop heartbeat age, active operations, queue depths, lock wait warnings, service health, and last shell action.

### 4.3 P1 Product Integrity Findings

#### PROD-01: The Browser is split between an iframe pane and an unchromed native window

The Dioxus Browser page admits that external sites cannot load in its iframe, then opens a separate top-level WebView. The in-app controls continue to model their own tab history and do not control that native window. This creates two browser states and makes reload, tabs, history, trust, and error handling inconsistent.

Required product architecture:

- `BrowserController` owns tabs, history, navigation state, permission state, trust state, and child/top-level WebViews.
- Dioxus renders Webizen chrome and sends typed browser commands.
- External sites render only in unprivileged native WebViews with no Tauri command bridge.
- Trusted local QApps receive a narrow capability object derived from their signed manifest and current grants.
- `qualia://` and `webizen://` semantic content may use purpose-built local views, but Browser state remains unified.
- Back, forward, reload, stop, open externally, downloads, popups, crashes, TLS errors, and navigation errors require real states and tests.

#### PROD-02: QApps presents a large static catalog rather than installed applications

The QApps catalog contains hundreds of hard-coded entries. Many are labelled Beta with `route: None`; their action opens a generic Studio editor. Counts such as Active and Beta therefore do not describe installed or runnable software.

Required correction:

- Replace the hard-coded runtime catalog with records from the signed QApp registry/package manager.
- Keep templates in a separate Create view and label them Templates.
- An installed QApp record must have package identity, version, signature/trust result, permissions, entry point, install/update state, and last launch result.
- Open, install, update, remove, inspect permissions, and repair must call real package operations.
- Hide aspirational catalog entries from the shipped product until they have packages and executable entry points.

#### PROD-03: WellFair mixes real host calls, phase labels, synthetic provenance, and placeholders

The WellFair shell exposes many phase-labelled areas, a fallback that says features are coming later, a synthetic `Host fixture` provenance hop, and demo status in the normal information architecture. It also renders several heavy panels at once for Health and Anatomy.

Required correction:

- Ship only implemented areas in the primary navigation.
- Move fixtures and demo data behind an explicit Demo workspace that can never be confused with the owner's vault.
- Generate provenance from real receipts or show an honest empty state.
- Mount one task-focused panel at a time and lazy-load expensive anatomy/render components.
- Make the first complete workflow Samsung Health import: select folder, preview sources, validate, import as a managed job, review results, inspect provenance, and export a report.

#### PROD-04: The home screen and global navigation are diagnostics-heavy

The app shell continuously displays backend/job status and offers many technical routes. Logs are a first-class route while Problems, recent human work, and recovery actions are not.

Required correction:

- Primary navigation: Home, WellFair, Library, Browser, QApps, Sanctuary, Work, Updates.
- Secondary navigation: Settings, Problems, Developer Tools.
- Home shows vault state, recent work, current operations, useful next actions, update availability, and problems requiring attention.
- MCP, SPARQL, raw telemetry, renderer probes, command harnesses, WAL inspection, and raw logs move to Developer Tools.

#### PROD-05: Version and release identity are inconsistent

`tauri.conf.json` packages version `0.0.24`, while `webizen-desktop` and `webizen-studio` Cargo packages report `0.0.23`. About dialogs use `CARGO_PKG_VERSION`, so installed builds can display a different version from the installer and updater feed.

Required correction:

- Define one release version source and validate it across Tauri config, Cargo packages, updater manifest, artifact names, About, and problem reports.
- Fail CI when any version differs.

### 4.4 P2 Maintainability Findings

- `commands/mod.rs` is a very large mixed-responsibility registry. Split commands by service boundary and generate one audited inventory.
- The old `shell_html.rs` iframe shell remains in the tree beside the current Dioxus shell. Mark it test-only or remove it after confirming no runtime dependency.
- Inline styles and repeated shell navigation code make consistent responsive behavior difficult. Establish a compact desktop design system and shared layout primitives.
- CSP currently allows broad inline/eval behavior in custom protocol responses. Tighten policies by content class and prohibit native capabilities in external browsing contexts.
- Settings portal, design studio, companion transport, and desktop status should not share one health concept. Each service needs an independent readiness result.

## 5. Target Ownership Model

### 5.1 `DesktopSupervisor`

Create `crates/webizen-desktop/src/supervisor.rs` as the owner of native services and operations.

It owns:

- service registry and lifecycle;
- cancellation tokens and join handles;
- bounded restart policy;
- operation registry and event broadcast;
- shutdown ordering;
- health and heartbeat snapshots;
- problem creation and recovery actions.

Core types:

```rust
ServiceId
ServiceState { Starting, Ready, Degraded, Failed, Stopping, Stopped }
OperationId
OperationKind
OperationState { Queued, Running, WaitingForUser, Cancelling, Completed, Failed, Cancelled }
OperationEvent { Started, Stage, Progress, Warning, Completed, Failed, Cancelled }
Problem { id, severity, service, operation_id, summary, detail, recovery_actions }
```

Services include settings API, companion gateway, daemon, host API, runtime kernel, persistence, native renderer, updater, reminders, telemetry, MCP, asset service, and browser controller.

### 5.2 Work Classification

- Native event loop: menu/tray dispatch, window state, and lightweight state publication only.
- Async I/O: network calls, async server handlers, updater checks, and event streams.
- Blocking I/O: import/export, archive, filesystem scans, WAL replay, PDF extraction, and package installation.
- CPU: parsing, report generation, certification, geometry, and heavy analysis.
- GPU: adapter initialization, renderer ownership, uploads, and frames.
- Service actor: serialized mutation of vault, host API, browser state, and daemon control.

Every command in the invoke registry receives one classification. CI fails if a new command is unclassified.

### 5.3 One Shell Command Bus

Menu, tray, keyboard, command palette, and Dioxus buttons dispatch the same typed action. The controller publishes the resulting route/window/operation state. No JavaScript injection plus event duplication is permitted.

### 5.4 Separate Network Planes

- Loopback API: settings, status, operations, problems, local Studio support. Bind only to loopback and authenticate browser sessions.
- Companion gateway: separate listener and protocol, disabled until pairing is enabled, capability-scoped and rate-limited.
- MCP: explicit Developer Mode or configured local integration, with its own bind policy and status.
- Daemon: internal service endpoint with no accidental exposure through permissive proxy routes.

### 5.5 Human Problem Reporting

Problems is the user surface; raw logs are supporting evidence.

Each problem shows:

- what failed;
- what the user was doing;
- whether their data was changed;
- whether the app recovered;
- actions such as Retry, Cancel, Restart Service, Open Folder, Roll Back Update, or Export Report.

Export Problem Report creates a redacted archive containing version/build identity, platform, WebView/GPU details, service states, operation timeline, crash markers, recent structured logs, updater state, route/action history, and a configuration summary with secrets removed.

## 6. Human UX Specification

### Home

- Continue recent work.
- Import health data.
- Open Library.
- Browse the web or semantic library.
- Open installed QApps.
- Current operations with progress and cancel controls.
- Problems requiring attention.
- Vault, offline, and update status in a quiet status area.

### WellFair

First release-complete flow:

1. Choose a Samsung Health export folder with a native folder dialog.
2. Scan in a blocking-I/O operation with file count and validation progress.
3. Preview recognized sources, date range, duplicates, unsupported files, and privacy destination.
4. Confirm import.
5. Import through the HostService without holding a UI-facing mutex.
6. Show records added, skipped, failed, and provenance receipts.
7. Open timeline and sleep/activity summaries backed by imported records.
8. Export a user-selected report through a managed operation.

No diagnosis language is introduced. Empty and failure states explain what is known and what action is available.

### Browser

- Webizen-owned chrome controls a real native browsing context.
- Tabs have stable ids and actual native WebView ownership.
- Back, forward, reload/stop, omnibox, open externally, downloads, and page errors work.
- External pages never receive native invoke privileges.
- Trusted QApps receive only manifest-approved capabilities.
- Trust and agent features build on the separate browser/trust plan, but basic browsing is release-blocking here.

### QApps

- Installed, Updates, and Discover/Create are separate views.
- Installed state comes from the package registry.
- Package signature, permissions, version, storage use, and last error are inspectable.
- Install, launch, update, remove, and repair are real managed operations.

### Updates

- Current version and channel.
- Last check and result.
- Available release notes.
- Download progress.
- Signature verification result.
- Restart now/later.
- Failure recovery and link to the problem report.

### Developer Tools

Developer Mode is off by default. When enabled it contains MCP, SPARQL, command inspection, raw logs, telemetry, render probes, controlled slow-operation tests, and controlled worker-panic tests. The controlled tests are compiled out of production or require a developer capability.

## 7. Implementation Work Packages

### WP0: Lock Down the Control Plane

Files:

- `settings_server.rs`
- `companion_gateway.rs`
- `capabilities/default.json`
- new control-plane integration tests

Deliverables:

- split listeners;
- loopback-only control API;
- remove generic HTTP-to-Tauri invocation;
- typed portal endpoints;
- paired companion authentication and capability checks;
- strict CORS/origin, request size, timeout, and rate limits.

Exit gate:

- external LAN request to control API fails;
- arbitrary browser origin fails;
- paired companion can complete only its documented operations;
- no regression to local settings or Samsung companion ingest.

### WP1: Repair Menus and Shell Ownership

Files:

- `shell/menu.rs`
- `main.rs`
- `webizen-studio/src/main.rs`
- new `shell/action.rs` and shell tests

Deliverables:

- typed action enum and single dispatch;
- remove duplicate direct eval/event paths;
- correct route table including GPU view;
- implement or remove every no-op menu item;
- operation acknowledgement and error presentation;
- menu enabled/checked state.

Exit gate:

- every enabled menu item has one tested result;
- no item only logs a debug message;
- one click produces one navigation or operation;
- keyboard shortcut and menu action produce the same state.

### WP2: Supervisor, Structured Logging, and Watchdog

Files:

- new `supervisor.rs`
- replace `desktop_log.rs`
- `main.rs`
- runtime, renderer, reminder, MCP, telemetry, updater, and server startup modules
- new Problems UI and operation status components

Deliverables:

- service/operation registry;
- cancellation and shutdown;
- non-blocking structured logs and rotation;
- UI and native event-loop heartbeats;
- watchdog snapshots;
- crash marker and problem report export;
- tray controls for Problems, log level, export report, and Developer Mode.

Exit gate:

- killing a worker produces a visible problem and bounded restart/failure state;
- app shutdown joins or cancels every managed service;
- the report identifies the last action and active operation;
- logging load does not stall menu response.

### WP3: Remove Blocking Paths and Coarse Locks

Files:

- `commands/mod.rs`, then split service command modules
- `settings_server.rs`
- `runtime.rs`
- `native_surface.rs`
- `med_reminder_notifier.rs`
- host API ownership in `main.rs`

Deliverables:

- command classification manifest;
- HostService actor;
- managed import/export/sync/backup/render operations;
- async or blocking-pool settings handlers;
- immutable protocol asset/frame snapshots;
- render-thread ownership and typed channel;
- no runtime `unwrap`/`expect` on shared locks.

Exit gate:

- a 60-second import does not delay navigation, status, cancel, or Problems UI;
- a poisoned worker state does not poison the shell;
- lock instrumentation reports no lock held across I/O or await;
- stress test concurrently runs status, import, render, and update check.

### WP4: Updater as a Product Service

Files:

- new `updater_service.rs`
- `main.rs`
- `shell/menu.rs`
- Updates UI
- desktop release workflow and updater manifest tests

Deliverables:

- one update implementation;
- delayed check after first paint;
- visible state/progress;
- signed artifact verification;
- restart flow and persisted failures;
- synchronized version source.

Exit gate:

- Windows installer updates from previous release to current release;
- Apple Silicon app updates from previous release to current release;
- interrupted download leaves the installed app runnable;
- invalid signature is rejected and shown as a problem;
- no update work blocks startup navigation.

### WP5: Human Shell and Problems UX

Files:

- `webizen-studio/src/main.rs`
- `components/dashboard.rs`
- new navigation, operations, problems, first-run, and empty-state components
- shared theme/layout styles

Deliverables:

- human primary navigation;
- useful Home;
- Problems and operation center;
- first-run storage/privacy/update choices;
- lazy route loading;
- diagnostics moved to Developer Tools;
- accessible keyboard/focus/reduced-motion behavior.

Exit gate:

- a fresh user can identify the next useful action without documentation;
- no raw diagnostic data dominates the first viewport;
- all primary actions are backed by real operations;
- 100%, 125%, 150%, and 200% Windows scaling remain usable.

### WP6: Complete Human Workflows

Files:

- WellFair shell, host client, health, sleep, medication, library, and report components
- Browser controller and browser UI
- QApp registry/package manager UI

Deliverables:

- complete Samsung import to report flow;
- real native browser with unified chrome/state;
- real installed QApp management;
- real recent Library/provenance view;
- remove synthetic provenance and placeholder product panels.

Exit gate:

- each workflow passes from a fresh installed profile;
- each failure has a recovery action;
- no workflow requires MCP or a raw command tool;
- displayed records and status are traceable to persisted state.

### WP7: Platform and Release Hardening

Files:

- desktop CI/release workflow
- packaging configuration
- smoke and end-to-end tests
- support and installer/updater documentation

Deliverables:

- Windows WebView2 and macOS WKWebView test matrix;
- Windows x64 installer/updater tests;
- macOS Apple Silicon DMG/updater tests with Metal capability report;
- clean install, upgrade, repair, uninstall, and data-retention checks;
- performance budgets and hang regression suite.

Exit gate:

- release fails on broken start route, menu contract, security boundary, updater feed, version mismatch, panic freeze, or missing artifact;
- release notes state real user changes and known limitations;
- rollback procedure is documented and exercised.

## 8. Test Strategy

### Unit and Contract Tests

- every shell action maps to exactly one handler;
- route ids are exhaustive;
- command classification is exhaustive;
- operation state transitions are valid;
- cancellation is idempotent;
- redaction removes secrets;
- version sources match;
- QApp status is capability-derived.

### Integration Tests

- loopback/LAN boundary and companion authentication;
- settings and status API timeouts;
- import while navigating and checking status;
- backup and restore cancellation semantics;
- daemon unavailable/restart;
- GPU unavailable/render panic/fallback;
- update available/download interrupted/signature invalid/restart later;
- previous crash marker shown on next launch.

### Desktop UI Smoke Tests

Run on Windows x64 and macOS arm64:

- app reaches first interactive paint;
- Home, WellFair, Library, Browser, QApps, Sanctuary, Work, Updates, Settings, and Problems load;
- every enabled native menu and tray item works;
- external site loads in the native browser and cannot invoke Tauri commands;
- slow operation keeps animations, navigation, and cancel responsive;
- controlled worker panic does not freeze the shell;
- window close, reopen from tray, and app quit complete cleanly.

### Performance Budgets

- first interactive shell is not gated on daemon, GPU, ontology seed, update, or host API readiness;
- menu acknowledgement target: under 100 ms;
- route transition target: under 150 ms before loading state is visible;
- status/Problems view remains responsive under heavy work;
- no synchronous filesystem write on the native event loop;
- no unbounded queue or unowned forever loop.

## 9. Delivery Order

The first implementation sequence is intentionally narrow and release-oriented:

1. WP0 control-plane lockdown.
2. WP1 menu/shell repair with an executable menu contract test.
3. WP2 supervisor, structured logs, operation model, and Problems view.
4. WP3 convert Samsung import, backup, sync, updater, protocol assets, and renderer first; then classify and migrate the remaining commands.
5. WP4 updater service and version unification.
6. WP5 human Home/navigation.
7. WP6 Samsung workflow, Browser, QApps, and Library.
8. WP7 cross-platform release hardening.

Do not begin a broad visual redesign before WP0-WP3. The redesigned UI must consume real service and operation state; otherwise it will reproduce the current static and diagnostic behavior with different styling.

## 10. Definition of Done

The overhaul is complete only when a person can install Webizen and:

1. Open a responsive Home immediately, even when daemon, GPU, update, or vault startup fails.
2. Use every enabled File, View, QApps, Tools, Help, and tray action with a visible result.
3. Import Samsung Health data, review persisted records and provenance, and export a report.
4. Browse an external site in a Webizen-controlled native browsing context without exposing native capabilities to that site.
5. Install, inspect, launch, update, and remove a real signed QApp.
6. Check, download, verify, and apply an update without blocking the app.
7. Cancel or recover from slow and failed operations.
8. See a useful Problem after a worker failure and export a redacted report.
9. Quit cleanly with all services stopped or joined.
10. Repeat the above on Windows x64 and macOS Apple Silicon release builds.

Until all ten are demonstrated by release tests, the app remains an in-progress desktop platform rather than a finished human product.
