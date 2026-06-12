# QApps Catalogue

_Review draft for current `qualia-studio` / Webizen Studio state_

## Purpose

This file inventories what we already have in the current Webizen Studio codebase, what exists only as a registered pane or placeholder component, and what larger Qapps still need to be added or ported.

## Status Legend

| Status | Meaning |
|---|---|
| `Shell` | Present as part of the studio environment, not a standalone Qapp |
| `Registered` | Present in `pane_registry.rs` and available in the palette |
| `Component` | Has a dedicated Dioxus component module in `src/components/` |
| `Mounted` | Actually mounted/rendered as a real component in the current canvas |
| `Operational` | Clearly backed by current daemon/studio behavior rather than only placeholder UI |

## What We Have Now

### 1. Studio Shell Capabilities

These are already part of the current Webizen Studio environment, even when they are not yet separate Qapps.

| Surface | Status | Notes |
|---|---|---|
| Workspace canvas | `Shell`, `Operational` | Point-grid / layered page workspace in `studio_canvas.rs` |
| Page navigation | `Shell`, `Operational` | Multi-page workspace with route-based pages |
| Property inspector | `Shell`, `Operational` | Shows pane geometry, bindings, layer info, supported views |
| Data binding panel | `Shell`, `Operational` | Placeholder SPARQL/N3 binding area |
| Live telemetry panel | `Shell`, `Operational` | SSE telemetry feed in right sidebar |
| Native engine status | `Shell`, `Operational` | Shows standalone WASM vs Webizen Server connected |
| Manifest deploy / rehydrate | `Shell`, `Operational` | `GET/POST /manifest` integration |
| Scoped theming | `Shell`, `Operational` | Environment/app/page/module theme layers |

### 2. Foundation Pane Library

These are mostly generic UI building blocks rather than full sovereign Qapps.

| Component ID | Display Name | Status | Notes |
|---|---|---|---|
| `card-view` | Card | `Registered` | Generic presentational pane |
| `details-view` | Expandable Details | `Registered` | Generic presentational pane |
| `progress-monitor` | Progress Bar | `Registered` | Generic presentational pane |
| `badge-indicator` | Badge / Status | `Registered` | Generic presentational pane |
| `rating-widget` | Rating | `Registered` | Generic presentational pane |
| `qr-code-display` | QR Code | `Registered` | Generic display primitive |
| `dynamic-form` | SHACL Form | `Registered` | Strong candidate for real form/Qapp workflows |
| `text-input` | Text Input | `Registered` | Generic input primitive |
| `text-area` | Text Area | `Registered` | Generic input primitive |
| `checkbox-toggle` | Checkbox | `Registered` | Generic input primitive |
| `switch-toggle` | Switch | `Registered` | Generic input primitive |
| `select-dropdown` | Select / Dropdown | `Registered` | Generic input primitive |
| `color-picker` | Color Picker | `Registered` | Useful for theming/editor tools |
| `range-slider` | Range Slider | `Registered` | Generic input primitive |
| `tab-group` | Tab Group | `Registered` | Layout primitive |
| `split-panel` | Split Panel | `Registered` | Layout primitive |
| `dialog-modal` | Dialog / Modal | `Registered` | Layout primitive |
| `divider` | Divider | `Registered` | Layout primitive |
| `image-comparer` | Image Comparer | `Registered` | Media utility |
| `carousel` | Carousel | `Registered` | Media utility |
| `avatar` | Avatar | `Registered` | Media utility |
| `alert-notification` | Alert / Notification | `Registered` | System utility |
| `spinner` | Spinner | `Registered` | System utility |
| `skeleton-loader` | Skeleton Loader | `Registered` | System utility |

### 3. Qapp-Shaped Modules Already Present

These are the most relevant current modules for a real Webizen Qapp catalogue.

| Component ID | Display Name | Status | Notes |
|---|---|---|---|
| `system-diagnostics` | Diagnostics & Telemetry | `Registered` | No dedicated component file found yet; studio shell already has telemetry/status panels |
| `error-logs` | Error & Audit Logs | `Registered` | Good candidate for unified audit/log Qapp |
| `sensor-data` | Sensor & IoT Stream | `Registered` | Good candidate for live data stream/monitor Qapp |
| `custom-web-module` | Web Module (RPC/IFrame) | `Registered`, `Operational` | Overlay/RPC metadata path exists; currently more of a host container than a finished Qapp |
| `neuro-symbolic-chat` | Neuro-Symbolic Chat | `Registered`, `Component` | Dedicated `chat_graph.rs`; current canvas still renders pane shell rather than mounting the component body |
| `llm-model-harness` | LLM Model Harness | `Registered`, `Component` | Dedicated `llm_harness.rs`; useful precursor to a real LLM Hub |
| `health-vital-monitor` | Health Vital Monitor | `Registered`, `Component` | Dedicated `health_monitor.rs`; domain Qapp placeholder exists |
| `personal-ontology-builder` | Personal Ontology Builder | `Registered`, `Component` | Dedicated `personal_ontology.rs`; domain Qapp placeholder exists |

### 4. Important Current Limitation

The current studio canvas does **not** yet mount most named custom Qapps as their real component bodies. In practice, the canvas still mainly renders pane wrappers, labels, geometry, and metadata. That means several entries above are best described as:

- palette-registered
- partially scaffolded
- not yet fully mounted/interactive in the live canvas

## Higher-Level Qapps Referenced Elsewhere But Not Yet Present In Current `qualia-studio`

These are explicitly named in project docs, but the referenced list comes from the older Flutter desktop surface rather than the present Dioxus studio implementation.

| Qapp / Surface | Current Webizen Studio Status | Notes |
|---|---|---|
| Dashboard | Missing | Usage overview, daemon health, active models |
| Group Chat + Sub-agents | Missing | Multi-agent session management |
| Chat to Qapp handoff | Missing | Important orchestration feature |
| Wallet | Missing | Multi-seed / credential-linked financial surface |
| Address Book / Directory | Missing | Contact / DID management |
| LLM Hub | Missing as full Qapp | Current harness exists, but not a full model manager |
| Ontology Hub | Missing | Needed for browse/import/share/seed flows |
| Qapp Vault | Missing | Install/list/launch qapps |
| Credential Manager | Missing | QCHK capability profile binding / DID sessions |
| Spatial Physics / Spatial screen | Missing | Relevant to future spatial Qapp mode |
| Settings | Missing as first-class Qapp | Studio still needs proper management surfaces |

## Recommended Additions

### Priority A: Core Management And Diagnostics

These should exist early because they make the whole environment manageable.

| Proposed Qapp | Why It Matters |
|---|---|
| `system_info` / System Info Center | Hardware, OS, storage, GPU/VRAM, Webizen Server state, active ports, build/runtime info |
| Diagnostics & Telemetry Center | Aggregate live logs, daemon health, model activity, extension status, performance counters |
| Audit / Error Log Viewer | Persisted logs, warnings, traps, ontology ingest failures, capability denials |
| Resource Manager | Unified management for models, ontologies, extension assets, downloaded artefacts |
| Extension / Capability Manager | Installed extensions, capability manifests, health, permissions, provisioning status |

### Priority B: Core Human Qapps

These are the sovereign day-to-day surfaces the platform needs.

| Proposed Qapp | Why It Matters |
|---|---|
| Chat Graph | Turn the existing chat pane into a real conversation/intent graph with handoff into other Qapps |
| Directory | Contacts, DIDs, organisations, trusted peers, agent identities |
| Wallet | Financial credentials, seed separation, DID-linked payment identities |
| Credential Manager | Capability profiles, signatures, sessions, trust relationships |
| Qapp Vault / Catalogue | Discover, install, update, launch, and permission-review Qapps |
| Settings | Studio/server preferences, identity, network, rendering mode, data directories |

### Priority C: Knowledge And Model Operations

| Proposed Qapp | Why It Matters |
|---|---|
| LLM Hub | Full GGUF lifecycle: download, activate, unload, residency, telemetry |
| Ontology Hub | Import, validate, ingest, inspect, share ontologies |
| Graph Explorer / Query Studio | Direct graph browsing, SPARQL/N3 tooling, shape inspection |
| Personal Ontology Studio | Extend the current ontology builder into a full editor/workbench |

### Priority D: Spatial / Advanced Interaction

| Proposed Qapp | Why It Matters |
|---|---|
| Spatial Workspace | Real `Spatial` presentation mode adapter |
| Node / Knowledge Graph View | Real `NodeRelational` adapter for relation-heavy work |
| 3D Scene / Full-Canvas Host | Proper host for immersive or simulation Qapps |

## Suggested Near-Term Build Order

1. System Info + Diagnostics Center
2. Qapp Vault / Catalogue
3. Directory
4. Wallet + Credential Manager
5. LLM Hub
6. Ontology Hub
7. Chat Graph as a true mounted, interactive Qapp

## Source Basis For This Catalogue

- `crates/qualia-studio/src/pane_registry.rs`
- `crates/qualia-studio/src/components/`
- `crates/qualia-studio/src/studio_canvas.rs`
- `docs/qapps_specification.md`
- `docs/manuals/ARCHITECTURE.md`
- `docs/manuals/developer-guide.md`
- `docs/release-targets.md`
