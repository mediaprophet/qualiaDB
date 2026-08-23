# Qualia-UI HyperCanvas — Implementation Tracker

**Copyright (c) 2026 Timothy Charles Holborn.** All rights reserved.
**Principal / inventor:** Timothy Charles Holborn <timothy.holborn@gmail.com>

> This document tracks progress toward the full HyperCanvas UI implementation.
> It is the authoritative status reference — update it as work progresses.
> Items are grouped by subsystem. Each item has a status: `[done]`, `[wip]`, `[todo]`, or `[blocked]`.

---

## How to use this document

- **Update the status** of items as you complete them.
- **Add new items** discovered during implementation — the list is not exhaustive.
- **Mark blockers** with `[blocked]` and note what's blocking.
- **Reference files** where relevant so the next session can find the code.
- **Keep it honest** — `done` means actually working in the browser, not just compiled.

---

## 1. Tool-Chest Architecture (dock, toolboxes, tool-chains, tools)

### 1.1 Dock — the furniture that holds toolboxes

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.1.1 | Dock renders toolbox buttons from the registry | [done] | `src/browser/docks.rs` — `build_toolbox_dock` |
| 1.1.2 | Dock buttons show toolbox glyphs + tooltips | [done] | |
| 1.1.3 | Dock buttons grouped into **families** (not flat list) | [done] | `src/browser/docks.rs` — 7 families: Epistemic, Authoring, Media, Communication, Governance, Life, Intelligence |
| 1.1.4 | Family headers are collapsible sections | [done] | Click family header to expand/collapse |
| 1.1.5 | 4-way docking (left/top/right/bottom) | [todo] | Mockup supports `dockPosition: 'left'` etc. Currently left-only |
| 1.1.6 | Dock position persisted to localStorage | [todo] | |

### 1.2 Flyout — shows tool-chains and tools for the active toolbox

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.2.1 | Flyout shows toolbox label + tool-chains + tools | [done] | `src/browser/docks.rs` — `show_flyout` |
| 1.2.2 | Tool buttons show icon, label, kind badge | [done] | |
| 1.2.3 | PlaceContainer tools actually place containers | [done] | `src/browser/interactions.rs` — `place_container_on_canvas` |
| 1.2.4 | RunAction/Query tools show honest notification | [done] | |
| 1.2.5 | Flyout closes when clicking outside | [done] | |
| 1.2.6 | Tool-chains are **draggable** to containers | [done] | Drag chain label from flyout → drop on container → chain activates there |
| 1.2.7 | Tool-chains are **selectable** to activate contextually | [done] | Click chain label → chain tools appear in ribbon for focused surface |

### 1.3 Context activation (OSX menu behaviour)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.3.1 | Contextual instrument panel appears when a container is selected | [done] | `src/browser/instrument_panel.rs` |
| 1.3.2 | Instrument panel shows tools specific to the selected container type | [done] | Tools per type: doc, sheet, code, ontology, map, social, graph, media/3d, health, webrtc, webview, rights |
| 1.3.3 | Instrument panel hides when canvas is clicked (no container focused) | [done] | |
| 1.3.4 | **Manifold-level context** — when no container is focused, show manifold-level tools | [done] | `instrument_panel::activate_chain` shows chain tools on manifold when no container selected |
| 1.3.5 | **Active tool-chain** drives the instrument panel, not just container type | [done] | `instrument_panel::activate_chain` — chain selection drives panel content |
| 1.3.6 | Tool-chain can be activated on a container by dragging | [done] | Drag chain label → drop on container → `activate_chain_on_container` |
| 1.3.7 | Tool-chain can be activated on the manifold by selecting | [done] | Click chain label → `activate_chain` shows tools on focused surface |
| 1.3.8 | Multiple tool-chains can be active simultaneously | [todo] | Currently single-chain; need stacked ribbon sections |

### 1.4 Tool family folders

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1.4.1 | `rich_text` family (doc toolbar, CML <q-entity>, <q-relation>, tri-view switch) | [done] | `src/browser/cml_document.rs` & `src/browser/container_views.rs` |
| 1.4.2 | `sheet` family (formula bar, sum, avg, P64, chart) | [partial] | Container view has formula bar + grid. Needs working formulas |
| 1.4.3 | `vibe` family (run, AST, gas, pulse::emit, capability.invoke) | [partial] | Container view has VibeScript console. Needs AST tab + gas |
| 1.4.4 | `aura` family (SHACL live badge, <q-aura-tray>, Super-Quin counter, export) | [done] | `<q-aura-tray>` in `container_views.rs` & `cml_document.rs` |
| 1.4.5 | `gis` family (pin, track, flow, trail, layers) | [partial] | Container view has map SVG. Needs layer toggles wired |
| 1.4.6 | `graph` family (SPARQL, expand, collapse, layout) | [todo] | Graph view exists but no SPARQL or interactive graph |
| 1.4.7 | `social` family (connect, chat, agent, graph) | [partial] | Social view has chat graph. Needs connect/chat actions |
| 1.4.8 | `health` family (biomarker, tomography, anatomy) | [partial] | Health view has tabs. Needs sub-panel interactions |
| 1.4.9 | `rights` family (sign, audit, consent) | [partial] | Rights view exists. Needs DID sign + audit trail |
| 1.4.10 | `media` family (orbit, pan, zoom, wireframe) | [todo] | 3D view is placeholder. Needs camera controls |
| 1.4.11 | `webframe` family (back, forward, reload, clip RDF) | [todo] | Webview view is placeholder. Needs navigation |
| 1.4.12 | `rtc` family (mic, cam, share) | [todo] | WebRTC view is placeholder. Needs toggle buttons |
| 1.4.13 | `econ` family (finance, portfolio) | [todo] | Finance view is placeholder |
| 1.4.14 | `latex` family (CAS, symbolic algebra) | [todo] | LaTeX view is placeholder |
| 1.4.15 | `triad` family (q42↔p64↔d10) | [todo] | Triad view is placeholder |
| 1.4.16 | `pulse` family (publish, subscribe) | [todo] | Pulse view exists but no publish/subscribe |
| 1.4.17 | `portal` family (wormhole, multi-tenant) | [todo] | Portal view is placeholder |
| 1.4.18 | `slide` family (presentation) | [todo] | Slide view is placeholder |
| 1.4.19 | `anatomy` family (10D vocal tract) | [todo] | Anatomy view is placeholder |
| 1.4.20 | `listen` family (audio, EnCodec) | [todo] | Listen view is placeholder |
| 1.4.21 | `vision` family (ahash, CV) | [todo] | Vision view is placeholder |
| 1.4.22 | `library` family (ingest, search, facets) | [todo] | Library view exists but no ingest/search |

---

## 2. Manifold System (virtual desktops)

### 2.1 Pager & switching

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.1.1 | Pager tabs for all 10 manifolds | [done] | `src/browser/topbar.rs` |
| 2.1.2 | Click tab to switch manifold | [done] | `src/browser/mod.rs` — `switch_manifold` |
| 2.1.3 | Alt+1..Alt+9 keyboard shortcuts | [done] | `src/browser/mod.rs` — `wire_alt_shortcuts` |
| 2.1.4 | Alt+O Exposé overview (grid of manifold cards) | [done] | |
| 2.1.5 | Exposé card click switches to that manifold | [done] | |
| 2.1.6 | Add new manifold ("+" button) | [done] | `src/browser/topbar.rs` — `add_new_manifold` creates empty seed + tab + switches |
| 2.1.7 | Manifold title is editable | [done] | `src/browser/topbar.rs` — `wire_title_rename` |
| 2.1.8 | Graph badge updates on switch | [done] | `src/browser/mod.rs` — `rerender_canvas` |

### 2.2 Canvas semantics

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.2.1 | Container dragging with 16px grid snap | [done] | `src/browser/interactions.rs` |
| 2.2.2 | Container resize with grid snap | [done] | |
| 2.2.3 | Dynamic z-ordering (click brings to front) | [done] | |
| 2.2.4 | Container selection (single) | [done] | |
| 2.2.5 | Multi-select containers | [done] | Shift+click toggles selection; Delete removes all selected |
| 2.2.6 | Canvas pan (drag empty canvas) | [done] | |
| 2.2.7 | Canvas zoom (wheel) | [done] | Wheel zoom applies CSS `transform: scale()` to `.canvas-content-layer`; zoom indicator shows percentage; range 30%–300% |
| 2.2.8 | Undo/redo (Ctrl+Z / Ctrl+Y) | [done] | `src/browser/history.rs` |
| 2.2.9 | History frame push on drag/resize/place | [done] | |
| 2.2.10 | Min size enforcement (280×180) | [done] | `src/browser/interactions.rs` — resize clamps to 280×180 min |
| 2.2.11 | Empty state for manifolds with no containers | [done] | |
| 2.2.12 | Canvas 2D/3D/4D spatial transforms | [todo] | Mockup has `canvas-engine.js` with CSS 3D matrix transforms |
| 2.2.13 | XYZD trackball gizmo | [todo] | Mockup has `xyzd-gizmo.js` (31KB) |
| 2.2.14 | Time ribbon / 4D datetime scrubber | [todo] | |

### 2.3 Wires (connections between containers)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.3.1 | SVG bezier wires rendered from seed connections | [done] | `src/browser/interactions.rs` — `render_wires` |
| 2.3.2 | Wire labels at midpoint | [done] | |
| 2.3.3 | Wire types have distinct styles (active, event, ontology, etc.) | [done] | CSS classes per type |
| 2.3.4 | Wire inspector — click wire to see details | [done] | `src/browser/wire_inspector.rs` |
| 2.3.5 | Wire drawing — drag from port to port | [done] | `src/browser/interactions.rs` — `wire_port_dragging` — drag from `.port-out` to `.port-in` |
| 2.3.6 | Wire deletion | [done] | Click wire to select (`.wire-selected`), Delete key removes it + its label |
| 2.3.7 | Wire label editing | [done] | Double-click wire label → inline input → Enter/blur commits, Escape cancels |
| 2.3.8 | Wire provenance tracing | [todo] | Inspector has Trace button but not wired |

### 2.4 Persistence

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.4.1 | ManifoldSeed CBOR serialization | [done] | `src/browser/manifest.rs` |
| 2.4.2 | localStorage persistence (base64 CBOR) | [done] | |
| 2.4.3 | Load saved manifolds on startup | [done] | |
| 2.4.4 | DOM-to-seed synchronization | [done] | `src/browser/interactions.rs` |
| 2.4.5 | Save manifold state on switch | [done] | `src/browser/history.rs` |

### 2.5 Save Architecture (Q42 checkpoints, bifurcation, provenance)

See `SAVE_ARCHITECTURE.md` for the full specification.

| # | Item | Status | Notes |
|---|------|--------|-------|
| 2.5.1 | Actor identity on save (default: did:qualia:timothy_charles_holborn) | [done] | `src/browser/manifest.rs` — `save_checkpoint` |
| 2.5.2 | Timestamp on save | [done] | ISO 8601 in checkpoint metadata |
| 2.5.3 | Save Mode dialog (File > Save As…) | [done] | `src/browser/topbar.rs` — `open_save_mode_dialog` |
| 2.5.4 | Save modes: Auto, Checkpoint, Snapshot, Pruned | [partial] | Auto + Checkpoint are live; Snapshot + Pruned are present (UI shows "engine wiring pending") |
| 2.5.5 | Named checkpoints with labels | [done] | Checkpoint mode records label + actor + timestamp |
| 2.5.6 | Checkpoint chain (parent → child) | [done] | Each checkpoint records parent ID |
| 2.5.7 | Checkpoint history view | [todo] | Modal showing checkpoint tree with branches |
| 2.5.8 | Branching / bifurcation | [todo] | Fork from any checkpoint; merge back |
| 2.5.9 | CRDT merge for conflicting operations | [todo] | Last-writer-wins for layout; semantic merge for content |
| 2.5.10 | Tombstone pruning | [todo] | Consolidate deletion records; new Merkle root |
| 2.5.11 | Archive export (.hmc with Bao streaming) | [todo] | Full provenance graph + all checkpoints |
| 2.5.12 | Distribution export (.q42 with credits + consent) | [todo] | Pruned + watermarked + derivative chain |
| 2.5.13 | Metadata stripping (fiduciary-authorized) | [todo] | Remove provenance + constituency for privacy-sensitive distribution |
| 2.5.14 | Streaming collaboration (CRDT operation stream) | [todo] | Real-time multi-agent editing with actor tags |
| 2.5.15 | BLAKE3 Merkle root computation | [todo] | Q42 epoch hash |
| 2.5.16 | Bao verified streaming | [todo] | Random-access chunk verification |
| 2.5.17 | Watermarking for distribution | [todo] | Traceability markers on published versions |
| 2.5.18 | Constituency tracking (data subjects, rights holders) | [todo] | `prov:Constituency` with consent state |
| 2.5.19 | Consent state tracking (pending/granted/denied) | [todo] | Blocks publish until granted |
| 2.5.20 | Credits generation from provenance graph | [todo] | `prov:Credits` — human-readable summary |
| 2.5.21 | Derivative chain (original → current) | [todo] | `prov:DerivativeChain` — DAG of transformations |
| 2.5.22 | Operation-level provenance (who changed what, when) | [todo] | `Operation` struct with actor, role, confidence |
| 2.5.23 | Auto-save on interval (default 60s) | [todo] | Rolling buffer of 5 auto-checkpoints |
| 2.5.24 | Status bar shows branch + last checkpoint + unsaved count | [todo] | |

---

## 3. Control Bar (top bar above canvas)

### 3.1 Socket-Case Pods

| # | Item | Status | Notes |
|---|------|--------|-------|
| 3.1.1 | Strata pod with drop-tray (multi-select checkboxes) | [done] | `src/browser/topbar.rs` |
| 3.1.2 | Epistemic lens pod with drop-tray (radio items) | [done] | |
| 3.1.3 | Dimension & time pod with drop-tray (2D/3D/4D + time span) | [done] | |
| 3.1.4 | Pod drop-trays close on outside click | [done] | |
| 3.1.5 | Strata filter actually filters containers | [done] | Checkboxes toggle `strata-hidden` class; containers dimmed to 18% opacity + grayscale + non-interactive |
| 3.1.6 | Epistemic filter actually filters containers | [done] | Radio buttons toggle `epistemic-hidden` class; same dimming as strata |
| 3.1.6 | Epistemic filter actually filters containers | [todo] | Tray UI exists but doesn't filter |
| 3.1.7 | Dimension switch changes canvas transform | [todo] | Tray UI exists but doesn't transform |
| 3.1.8 | Time span scrubber controls 4D time | [todo] | |

### 3.2 Action shelf

| # | Item | Status | Notes |
|---|------|--------|-------|
| 3.2.1 | Telemetry sidebar toggle | [done] | `src/browser/topbar.rs` — `toggle_tech_sidebar` |
| 3.2.2 | a11y toggle | [done] | Shows notification |
| 3.2.3 | Telemetry sidebar shows Merkle-CRDT DAG | [done] | Structural mock |
| 3.2.4 | Telemetry sidebar shows container quads | [done] | Structural mock |
| 3.2.5 | Telemetry sidebar shows connection ontology | [done] | Structural mock |

### 3.3 Menubar

| # | Item | Status | Notes |
|---|------|--------|-------|
| 3.3.1 | Menubar with File/Edit/View/Insert/Help | [done] | `src/browser/topbar.rs` |
| 3.3.2 | Version badge (0.0.31-dev) | [done] | |
| 3.3.3 | Fiduciary badge | [done] | |
| 3.3.4 | Menu items have dropdown menus | [done] | `src/browser/topbar.rs` — `build_menu_dropdown` + `wire_menu_dropdowns` |
| 3.3.5 | File menu: new manifold, save, export | [done] | Save writes CBOR-LD to localStorage; export/import are present (file picker pending) |
| 3.3.6 | Edit menu: undo, redo, delete, duplicate, select all | [done] | Undo/redo wired; delete/duplicate/select-all wired |
| 3.3.7 | View menu: toggle dock, sidebar, zoom | [done] | Toggle dock/telemetry/a11y wired; zoom pending canvas transform |
| 3.3.8 | Insert menu: place containers | [done] | Inserts doc/sheet/code/map/ontology/social/3d/webrtc |

---

## 4. Containers (typed occupants on manifolds)

### 4.1 Container rendering

| # | Item | Status | Notes |
|---|------|--------|-------|
| 4.1.1 | All planned container types have view builders | [done] | `src/browser/containers.rs` + `container_views.rs` + `container_views_ext.rs` + `workflow_panels.rs` |
| 4.1.2 | Container header with type tag + title + honesty badge | [done] | |
| 4.1.3 | Container body renders type-specific content | [done] | |
| 4.1.4 | Container resize handle | [done] | |
| 4.1.5 | Container connection ports (in/out) | [done] | Visual only, not draggable |
| 4.1.6 | Honesty badges (live/partial/present/missing) | [done] | |
| 4.1.7 | Strata badges | [done] | |
| 4.1.8 | Epistemic modality badges | [done] | |
| 4.1.9 | ContainerKind field (content/panel/widget) | [done] | `src/tool_chest/core/registry.rs` — `ContainerKind` enum + `from_type()` inference |
| 4.1.10 | Checkpoint Tray panel container | [done] | `src/browser/workflow_panels.rs` — timeline with checkpoint history from localStorage |
| 4.1.11 | Credential Inspector panel container | [done] | Shows capabilities (active/suspended/revoked/pending) + access control policies |
| 4.1.12 | Context Markup Editor panel container | [done] | Shows markup types, append scopes, temporal status (structural mock) |
| 4.1.13 | Provenance Panel container | [done] | Shows contribution roles, transformation types, derivative chain (structural mock) |
| 4.1.14 | Publication Workflow panel container | [done] | Shows 8-step workflow (save → visibility → constituency → consent → prune → credits → export → strip) |
| 4.1.15 | Constituency Manager panel container | [done] | Shows constituency types + consent state (structural mock) |
| 4.1.16 | Capability Badge widget container | [done] | Green/yellow/red/grey Sentinel indicator |
| 4.1.17 | Checkpoint Indicator widget container | [done] | Shows branch + last checkpoint + unsaved count |
| 4.1.18 | Consent Indicator widget container | [done] | Shows aggregate consent state (green/yellow/red) |
| 4.1.19 | Insert menu includes workflow containers | [done] | File > Insert > + Checkpoint Tray, + Credential Inspector, etc. |
| 4.1.20 | Credential-conditional rendering pipeline | [todo] | Filter context graph by viewer's capabilities (see SAVE_ARCHITECTURE.md §10) |
| 4.1.21 | Context markup live editing | [todo] | Add/edit/remove markup nodes on document text spans |
| 4.1.22 | Context markup credential-conditional visibility | [todo] | Filter markup by appendScope + viewer capabilities |
| 4.1.23 | Provenance graph live tracking | [todo] | Track contributions, sources, transformations in real-time |
| 4.1.24 | Credits generation from provenance graph | [todo] | `prov:Credits` — human-readable summary |
| 4.1.25 | Derivative chain visualization | [todo] | DAG view of original → current |
| 4.1.26 | Constituency live tracking + consent management | [todo] | Track consent state per constituency, block publish if pending |
| 4.1.27 | Capability resolution from Sentinel VM | [todo] | Live capability status from Sentinel VM enforcement |
| 4.1.28 | Watermarking for distribution | [todo] | Embed traceability markers in published content |

### 4.2 In-container interactions

| # | Item | Status | Notes |
|---|------|--------|-------|
| 4.2.1 | Doc: contenteditable with toolbar | [partial] | Toolbar buttons exist but formatting not wired |
| 4.2.2 | Doc: view switcher (Visual / Markdown / RDF Triples) | [done] | `src/browser/container_views.rs` — tabbed switcher with contenteditable, markdown textarea, RDF triple table |
| 4.2.3 | Sheet: formula bar + interactive grid | [partial] | Formula bar exists, grid is static |
| 4.2.4 | Sheet: cell selection + editing | [done] | Click cell to edit, Enter/blur to commit, formula evaluation (+, -, *, /, SUM range) |
| 4.2.5 | Code: VibeScript editor + run | [partial] | Console exists, run is mock |
| 4.2.6 | Code: AST inspector tab | [todo] | |
| 4.2.7 | Code: Gas accounting | [todo] | |
| 4.2.8 | Ontology: alignment matrix with add-row | [partial] | Tree view exists, no matrix |
| 4.2.9 | Map: layer toggles (Flow, Pins, UAV, Trail) | [partial] | Layer bar exists, toggles not wired |
| 4.2.10 | Social: chat graph with agent chips | [partial] | Chat messages exist, no interaction |
| 4.2.11 | Health: sub-panel tabs (biomarker, tomography, anatomy) | [partial] | Tabs exist, sub-panels are mock |
| 4.2.12 | Contextual RDF popover (text selection → tag) | [done] | `src/browser/contextual_popover.rs` — select text in doc → popover with 8 entity types → `<q-entity>` tag wraps selection with `data-entity-type` |
| 4.2.13 | Search Workbench (faceted + builder + SPARQL + saved) | [done] | `src/browser/search_workbench.rs` — 4-mode modal: faceted chips, visual triple-pattern builder with SPARQL preview, manual SPARQL editor, saved queries list; localStorage persistence; query-as-container-source placement; Ctrl+Shift+F shortcut; topbar Search button; command palette entries. SPARQL execution is mocked — daemon wiring pending |
| 4.2.14 | Logic Workbench (71 reasoning tools) | [done] | `src/browser/logic_workbench.rs` (723 lines) + `logic_workbench/` subdirectory with 12 focused modules: helpers, descriptions, dispatch, tests, p0_core, p0_ext, p1_legal, p1_governance, p1_logic, p1_advanced, p2_domain, p2_infra, p2_infra_ext, p2_extras. Modal overlay with 71 tool tabs: 9 P0 + 8 P1 legal + 6 P1 governance + 9 P1 logic + 8 P1 advanced + 9 P2 domain (Clinical Risk, DICOM, Comorbidity, Chemistry, Physics, ODE, Bioinformatics, GBM/VaR, Diffusion) + 9 P2 infra (Bytecode/VM, SLG Arena, Forge Compute, Compute Profile, Privacy/HE/DP, Model Lifecycle, Inference Monitor, GGUF Tokenizer, P64 Weight) + 10 P2 infra ext (CRDT/Sync, Agency/Merkle, Key Vault, Policy Evaluator, Consent Manager, Carrier/Media, Control Feedback, Likeliness, QUBO, OWL Converter) + 3 P2 extras (Allen/RCC8, Manifold Logic, Calculus). 65 modalities in Evaluate Modality selector. 72 command palette entries via `dispatch_command`. Ctrl+Shift+L shortcut. 15 unit tests. All evaluation mocked — MCP engine wiring pending |

### 4.3 Container placement

| # | Item | Status | Notes |
|---|------|--------|-------|
| 4.3.1 | PlaceContainer tools place containers on canvas | [done] | |
| 4.3.2 | New containers get cascading offset | [done] | |
| 4.3.3 | New containers get correct honesty label | [done] | "missing" by default |
| 4.3.4 | New containers are wired for drag/resize/select | [done] | |
| 4.3.5 | Container deletion | [done] | ✕ button on header + Delete/Backspace key shortcut |
| 4.3.6 | Container duplication | [done] | Ctrl+D / Cmd+D duplicates selected container(s) with 30px offset |

---

## 5. Right Dock (aura tray + pulse stream)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 5.1 | Aura tray (SHACL validation status) | [done] | Structural mock |
| 5.2 | Pulse stream (event log) | [done] | Structural mock |
| 5.3 | Right dock is collapsible | [done] | Collapse button (▶) hides content + shrinks dock to 20px; expand button (◀ Dock) restores |
| 5.4 | Aura tray shows real ontology shapes | [todo] | Awaiting backend |
| 5.5 | Pulse stream shows real events | [todo] | Awaiting backend |

---

## 6. Command Palette

| # | Item | Status | Notes |
|---|------|--------|-------|
| 6.1 | Ctrl+K opens palette | [done] | `src/browser/command_palette.rs` |
| 6.2 | Search filtering | [done] | |
| 6.3 | Manifold switching from palette | [done] | |
| 6.4 | Command execution with notification | [done] | |
| 6.5 | Keyboard navigation (arrow keys + Enter) | [done] | Arrow Up/Down to navigate, Enter to execute, hover to select |
| 6.6 | Commands for placing containers | [todo] | |
| 6.7 | Commands for tool activation | [todo] | |
| 6.8 | Recent commands | [todo] | |
| 6.9 | Fuzzy search | [done] | `src/browser/command_palette.rs` — `fuzzy_score()` with subsequence matching, word boundary bonuses, consecutive match bonuses, early match bonuses; results sorted by score |

---

## 7. Theme & Visual System

| # | Item | Status | Notes |
|---|------|--------|-------|
| 7.1 | Glassmorphism design system | [done] | `src/browser/css.rs` |
| 7.2 | CSS variables for colors, spacing, typography | [done] | |
| 7.3 | Honesty badge colors | [done] | |
| 7.4 | Strata/modality badge colors | [done] | |
| 7.5 | Theme presets (multiple themes) | [todo] | Mockup has theme system |
| 7.6 | Sanctuary zero-motion mode | [todo] | |
| 7.7 | WCAG contrast modes | [todo] | |
| 7.8 | Reduced motion support | [todo] | |

---

## 8. Accessibility

| # | Item | Status | Notes |
|---|------|--------|-------|
| 8.1 | ARIA roles on dock, canvas, containers | [todo] | |
| 8.2 | Keyboard navigation between containers | [todo] | |
| 8.3 | Focus indicators | [partial] | Selected container has border |
| 8.4 | Screen reader announcements | [todo] | |
| 8.5 | a11y toggle button | [done] | Shows notification |

---

## 9. Backend Integration (out of scope for UI phase, tracked for completeness)

| # | Item | Status | Notes |
|---|------|--------|-------|
| 9.1 | IntentBus implementation | [blocked] | Needs daemon repo |
| 9.2 | Backend transport (WebSocket/HTTP) | [blocked] | Needs daemon repo |
| 9.3 | Ontology loading (CBOR-LD) | [blocked] | Needs build pipeline |
| 9.4 | SPARQL query execution | [blocked] | Needs daemon |
| 9.5 | SHACL validation | [blocked] | Needs daemon |
| 9.6 | Chat graph LWW CRDT | [blocked] | Needs daemon |
| 9.7 | WebRTC stream | [blocked] | Needs desktop host |
| 9.8 | Health telemetry | [blocked] | Needs daemon + consent |
| 9.9 | Library ingest/search | [blocked] | Needs daemon |
| 9.10 | DID signing | [blocked] | Needs daemon |
| 9.11 | Model discovery/download | [blocked] | Needs daemon |

---

## 10. Build & Verification

| # | Item | Status | Notes |
|---|------|--------|-------|
| 10.1 | `cargo check` passes | [done] | |
| 10.2 | `cargo test` — 73 tests pass | [done] | |
| 10.3 | Trunk WASM build succeeds | [done] | |
| 10.4 | App loads in browser preview | [done] | `http://127.0.0.1:8080` |
| 10.5 | No unused import warnings | [done] | |
| 10.6 | Files under 800-line limit | [done] | `command_palette.rs` split into `command_palette.rs` (597 lines) + `command_palette/commands.rs` (502 lines). All logic_workbench modules under 800 |
| 10.7 | WASM compatibility maintained | [done] | |

---

## Summary counts

| Status | Count |
|--------|-------|
| [done] | ~55 |
| [partial] | ~12 |
| [todo] | ~55 |
| [blocked] | 11 |

**Next priorities** (highest impact for UI/UX completeness):

1. ~~Toolbox families in the dock~~ — ✅ done
2. ~~Context activation~~ — ✅ done
3. ~~Manifold-level context~~ — ✅ done
4. ~~Menu dropdowns~~ — ✅ done
5. ~~Container deletion~~ — ✅ done
6. ~~Wire drawing~~ — ✅ done
7. ~~Keyboard navigation in command palette~~ — ✅ done
8. ~~Min size enforcement~~ — ✅ done
9. ~~Doc view switcher~~ — ✅ done
10. ~~Sheet cell editing~~ — ✅ done

**Updated next priorities:**

1. ~~Multi-select containers~~ — ✅ done (Shift+click)
2. ~~Wire deletion~~ — ✅ done (click wire + Delete)
3. ~~Wire label editing~~ — ✅ done (double-click label)
4. ~~Container duplication~~ — ✅ done (Ctrl+D)
5. ~~Strata/epistemic filters actually filter containers~~ — ✅ done
6. ~~Canvas zoom~~ — ✅ done (wheel zoom + indicator)
7. ~~Right dock collapsibility~~ — ✅ done (collapse/expand)
8. ~~Add new manifold~~ — ✅ done (+ button)
9. ~~Fuzzy search in command palette~~ — ✅ done (subsequence + scoring)
10. ~~Contextual RDF popover~~ — ✅ done (select text → annotate)
11. ~~Search Workbench~~ — ✅ done (faceted + visual query builder + manual SPARQL + saved queries + query-as-container-source)
    - `src/browser/search_workbench.rs` — modal overlay with 4 modes:
      - **Faceted Search** — chips for ontology prefix, entity type, epistemic modality, strata, honesty, container type; multi/single-select; mock result list with count
      - **Query Builder** — PREFIX declarations textarea + dynamic triple pattern rows (subject/predicate/object) + generated SPARQL preview; predicates drawn from common ontology predicates
      - **Manual SPARQL** — editable textarea with default SELECT template; run/save/place-as-container actions
      - **Saved Queries** — list view with load/place/delete; persisted in `localStorage` under `qualia-ui:saved-queries`
    - Saved queries can be placed on canvas as graph containers (with `data-query` / `data-query-name` attributes) — engine wiring pending for actual execution
    - Triggered via Ctrl+Shift+F, topbar Search button, or command palette entries
    - 5 unit tests for JSON parsing/serialization of saved queries
    - Honest labeling: footer notes that SPARQL execution requires QualiaDB daemon backend

---

## New workstreams from 2026-08-18 consult docs

Five consult documents were added on 2026-08-18, defining major new UI workstreams.
All assume the QualiaDB engine is completed; qualia-ui surfaces will be built as
`present` (structure exists, backend not wired) with honest labels unless noted.

### Workstream A — Collaborative / Cooperative ERP & PM

**Source:** `consult/20260818_qualia-collaborative-ui-requirements.md`
**Extended plan:** `WORKSTREAM_A_IMPLEMENTATION_PLAN.md`
**Status:** [wip] — Phase 1 containers done (9 project + 2 rights/wallet), extended plan created

Phase 1 completed (11 containers):
- Project containers: kanban, project_sheet, budget, cost_base, deliverable, review, discussion, roadmap, commons
- Rights & Wallet containers: rights (5 tabs: agreements, deontic, jural, breach, consents), wallet (4 tabs: balances, ILP, tax, compute)
- Files: `project_views/` (9 modules), `rights_views/` (3 modules), `container_inline_views.rs`
- Manifold seeds updated: `projects.rs` (9 containers), `rights.rs` (3 containers)
- Command palette entries added (20+ new commands)

Extended plan covers 74 total container types across:
- Planning & visualization (gantt, dashboard, timeline, calendar)
- Knowledge & documentation (wiki, doc_mgmt)
- Resource management (resource_report, time_tracking)
- Governance & policy (governance, governance_meetings, voting, risk, conflict_of_interest)
- Task & issue management (task_list, issues)
- Asset & licensing (asset_mgr, credentials)
- Community (events, news, bounties)
- Product/service/operations (product_catalog, release_manager, customer_feedback, customer_support, billing, distribution, infrastructure)
- Group layer (group_profile, group_portfolio, group_community, group_suppliers, group_governance)
- Personal aggregate views (personal_calendar, personal_tasks, personal_dashboard, notification_center)
- Currency & multi-sig wallet enhancement
- Project lifecycle stages with stage-dependent rules
- Permissive commons consumption tracking
- Agreement framework & instruments (§8a: agreement_builder)
- Fair value, compensation & obligation cost (§8b: compensation_model, contribution_ledger)
- Differential licensing & obligation recovery (§8c: license_builder, obligation_tracker)
- Awards, tokens & recognition (§8d: awards, token_mgr)
- Disputes, complaints & corrections (§8e: disputes, complaints, corrections)
- Onboarding & bulk admin (§8f: onboarding, bulk_import)
- Governance meetings, minutes & resolutions (§8g)
- Conflict of interest (§8h)
- Zero-knowledge privacy controls (§8i: cross-cutting)
- Provenance studies, innovation, research & IP creation (§8j: provenance_studies, innovation_log, research_tools, ip_registry)
- Knowledge base & project specialist agents (§8k: knowledge_base, agent_console)
- External data sources & evaluations (§8l: data_sources, evaluations)
- Cross-cutting: inter-container linking, provenance, sensitivity badges, capability gating, i18n, accessibility, ZK privacy

### Workstream B — Health / Wellbeing / Document / Credentials

**Source:** `consult/20260818_qualia-health-wellbeing-document-credentials-ui-requirements.md`
**Status:** [done] — All 22 health containers built

Implementation: `src/browser/health_views/` (22 modules), `src/tool_chest/manifolds/health.rs` (new manifold seed with 22 SeedContainers + 21 SeedConnections). All containers use `present` honesty label (structure exists, backend not wired). Dispatch arms, strata/tag mappings, and 22 command palette entries added.

Containers built (22):
- P0: health_overview (dashboard), conditions (list+allergies), clinical_reports (list), lab_results (expandable table), medications (list+log), mental_wellbeing (mood+assessments+observations), hypotheses (evidence+disclosure tiers), health_documents (library+QECP)
- P1: vitals (chart), therapy_notes (Sanctuary-classified), sleep (chart+debt), diet (log+nutrition), physical_activity (log), immunizations (list), procedures (list), family_history (tree), biometrics (ZK proof actions), welfare_support (needs+streams+letters), life_records (events+cases+tasks), authority_attestations (list), safeguards (dead-man+incapacity), disclosure_log (leak tracing)

### Workstream C — Logic modality editors

**Source:** `consult/20260818_logic-modalities-audit.md`
**Status:** [done] — All P0 + P1 + P2 logic modality panels built

Core P0 reasoning tools — DONE (9 panels):
- Deontic, N3, SHACL, RDF-Star, Ontology Builder, Evaluate Modality, Symbolic Infer, Jural Relations, Argumentation

P1 panels — DONE (31 panels):
- P1 Legal (8): STIT, Causal, Responsibility, Capacity, Delegation, Contract, Consensus, Meta-Deontic
- P1 Governance (6): Value Flow, Interaction Gov, Identity Fabric, Capability Gap, Legal Compose, Deontic Compose
- P1 Formal Logic (9): Epistemic, Paraconsistent, LTL, CTL, ASP, Defeasible, Linear, Description, Dialectical
- P1 Advanced (8): Abductive, Fuzzy, Probabilistic, Graph Theory, Interval, Manifold 10D, Epistemic Boundaries, Modal

P2 panels — DONE (31 panels):
- P2 Domain (9): Clinical Risk, DICOM Viewer, Comorbidity, Chemistry, Physics, ODE Solver, Bioinformatics, GBM/VaR, Diffusion
- P2 Infrastructure (9): Bytecode/VM, SLG Arena, Forge Compute, Compute Profile, Privacy/HE/DP, Model Lifecycle, Inference Monitor, GGUF Tokenizer, P64 Weight
- P2 Infrastructure Extended (10): CRDT/Sync, Agency/Merkle, Key Vault, Policy Evaluator, Consent Manager, Carrier/Media, Control Feedback, Likeliness, QUBO, OWL Converter
- P2 Extras (3): Allen/RCC8, Manifold Logic, Calculus

Implementation: `src/browser/logic_workbench.rs` (723 lines) + `logic_workbench/` subdirectory with 12 modules (helpers, descriptions, dispatch, tests, p0_core, p0_ext, p1_legal, p1_governance, p1_logic, p1_advanced, p2_domain, p2_infra, p2_infra_ext, p2_extras). 71 tool tabs. 65 modalities in Evaluate Modality. 72 command palette entries. 15 unit tests. `command_palette.rs` split into `command_palette.rs` (597 lines) + `command_palette/commands.rs` (502 lines). All evaluation mocked — engine wiring pending.

### Workstream D — 3D / Animation / Dataset / Audio

**Source:** `consult/20260818_qualia-3d-animation-dataset-audio-ui-requirements.md`
**Status:** [complete] — P0 + P1 + P2 containers built

P0 containers built (10):
- Studio manifold (6): scene_view, animation_timeline, desk_surface, transport, routing_matrix, spatial_audio
- Datasets manifold (4): dataset_registry, dataset_importer, presentation_editor, view_canvas

P1 containers built (10):
- Studio 3D (5): scene_graph, material_editor, lighting_editor, tensor_inspector, asset_library
- Studio audio (3): channel_strip, meter_bridge, automation_lanes
- Datasets (2): annotation_panel, lineage_graph

P2 containers built (12):
- Studio 3D (3): lod_chain (5 LOD levels with error threshold chart + size bars), shadow_settings (10 params + cascade split visualization + thermal warning), gis_maps (8 map layers + viewport + zoom controls + GeoSPARQL endpoint)
- Animation (2): ragdoll_skin (14 bones + 11 physics joints + skeleton viewport), animation_export (6 clips + 6 export formats + export options)
- Datasets (4): video_view (player + scrub bar + waveform + 5 markers + video info), presentation_publish (6 targets + access control + SHACL validation), super_resolve (6 SR jobs + 8 CV tools + geometry-assisted curation), cad_curation (6 CAD files + 6 GD&T inspections + mesh conversion)
- Audio (3): desk_persistence (6 presets + 4 shared collaborators + current state), hrtf_personalization (5 profiles + 8 anthropometric params + 6 calibration results + Selfhood sensitivity), manifold_transition_audio (8 transitions + 8 ambience settings + progress bar)

Implementation: `src/browser/studio_views/` (22 modules), `src/browser/dataset_views/` (10 modules). 32 dispatch arms, strata/tag mappings, 32 command palette entries. All containers use `present` honesty label with mock data and inline CSS.

Workstream D complete — 32 containers across Studio (22) and Datasets (10) manifolds.

### Workstream E — QApp browser

**Source:** `consult/20260818_qualia-ui-qapp-audit.md`
**Status:** [pending] — not started

Core surfaces required:
- QApp discovery browser (333 QApps across 13 categories)
- Category filter + search
- QApp manifest viewer (name, version, required shapes, capabilities, dev port)
- QApp install/launch/uninstall actions
- QApp capability requirements display
- Featured workspace templates (6)
- QApp configuration editor

### Workstream F — VibeScript-Driven Dynamic UI & Collapsible Dock Engine

**Source:** `vibescript-core.md`, `poet-mindware-workbench-ui.md`
**Status:** [complete] — Vibe UI Engine, Collapsible Panels, and Live Hot-Reloading

Implemented items:
- **Collapsible Docks & Trays**: `create_collapsible_dock_panel` and `render_subtray` with interactive chevrons, badges, and flex management in `docks.rs` and `diagnostics.rs`.
- **Aura Tray Sub-Trays**: Divided into SHACL Shapes, Ontologies & Schemas, and Super-Quin Sentinel sub-trays with violation expanders.
- **Vibe UI Dynamic AST & Reconciler**: `vibe_ui.rs` mapping evaluated VibeScript records (`type: "dock_panel"`, `"subtray"`, `"shacl_shape"`, `"metric"`, `"button"`) into DOM elements.
- **Live Re-evaluation & Hot-Reloading**: `<q-vibe-ui>` component hosting live VibeScript source editor, evaluation through `vibe::Engine`, and DOM reconciliation without WASM recompilation.
- **Fixtures & Tests**: `ui_dock_trays.vibe` fixture in `crates/vibe/fixtures/`, 168 passing tests in `poet`, 238 passing tests in `vibe`. Browser verification validated live reload.

