# POET / Webizen Lower-Cost Agent Execution Playbook

**Date:** 2026-09-04  
**Audience:** GPT-5.6 Luna and GPT-5.5 coding sessions  
**Purpose:** Complete the bulk of POET, health, portable-app, and Webizen Desktop work economically without creating a larger expert-repair bill.  
**Authority:** This playbook sequences work. `AGENTS.md` and the canonical specifications remain authoritative when they conflict with it.

## 1. Recommended model strategy

Use **GPT-5.6 Luna with high reasoning** for the default implementation queue. Its work must remain inside the bounded task packets marked `LUNA`. Use **GPT-5.5 with medium reasoning** for packets marked `5.5` because they cross security, storage, app-host, licensing, or ABI boundaries.

If only one model will be used for every packet, choose **GPT-5.5 medium**. Luna is the cost-saving default only when the routing and stop rules below are followed.

Do not raise reasoning effort merely because a task is failing. After two materially different failed attempts, stop and record the blocker. Repeated retries are exactly the token-cost failure this playbook is intended to prevent.

| Marker | Default model | Suitable work |
|---|---|---|
| `LUNA` | GPT-5.6 Luna, high | Local UI restoration, typed projections, focused tests, CSS, honest states, narrow adapters, documentation |
| `5.5` | GPT-5.5, medium | Cross-crate contracts, permissions, consent enforcement, package lifecycle, ingestion architecture, bounded parser design |
| `REVIEW` | Return to the expert reviewer | Cryptographic key handling, clinical claims, licence ambiguity, ABI changes, destructive migrations, unresolved architecture conflicts |

## 2. Intended outcome

The programme is successful when:

1. POET is a coherent, human-usable spatial application environment rather than a dense technical demonstration.
2. POET manifolds and containers are projections of portable apps and shared Qualia capabilities, not private implementations.
3. Health is person-controlled, provenance-rich, consent-aware, and useful without making fabricated clinical claims.
4. Webizen Desktop is a general multi-app native host and advanced administrative control plane. POET is one hosted app, not the shell's hard-coded purpose.
5. The generic-view delegation count falls from the current ceiling of 112 without deleting useful workflows or relabelling CRUD as complete.
6. Every completion claim has task-level evidence, including real persistence, failure states, accessibility, and browser/native verification where applicable.

## 3. Canonical reading order

Every new session reads:

1. `AGENTS.md` in full.
2. Sections 1–8 of this playbook plus the assigned task packet only.
3. `docs/POET_IMPLEMENTATION_SESSION_LEDGER.md`.
4. Only the canonical specification named in that packet.
5. Only the implementation files named in that packet, plus directly imported dependencies required to understand them.

Do not reread the whole repository, every tracker, or every POET specification in each session. That wastes tokens and increases contradictory interpretations.

Canonical references:

- Product integrity: `docs/POET_PRODUCT_INTEGRITY_REMEDIATION_2026-08-29.md`
- Programme architecture: `docs/POET_WEBIZEN_HEALTH_PLATFORM_2026-09-04.md`
- Master status: `docs/poet/MASTER_TRACKER.md`
- POET architecture: `docs/poet/00_ARCHITECTURE_OVERVIEW.md`
- Human UX: `docs/poet/01_HUMAN_CENTRIC_UX_SPEC.md`
- Health: `docs/poet/06_HEALTH_PERSON_RECORDS_SPEC.md`
- Desktop host: `docs/poet/08_DESKTOP_ADMIN_HOST_SPEC.md`
- Backend limitations: `docs/POET_UI_BACKEND_GAPS_2026-08-28.md`

## 4. Non-negotiable implementation rules

The agent must preserve all `AGENTS.md` constraints and these product rules:

- Never replace a domain workflow with a generic record form.
- Never present static, sampled, default, seeded, deterministic, or fabricated operational data as live.
- Never increase `GENERIC_DELEGATION_CEILING` to make a test pass. The ceiling may only decrease when a thin delegation is genuinely removed.
- Never mark a master-tracker requirement `[x]` without task-level UAT. Use `[~]` for a useful but incomplete slice.
- Never invent a daemon endpoint, capability ID, record field, package format, licence, consent rule, or clinical interpretation. Find the real contract or stop.
- Never render persisted/user/network text with unescaped `inner_html`. Static trusted templates are permitted; dynamic values use `text_content` or safe DOM APIs.
- Never ask people to enter raw JSON, hashes, integer booleans, or ISO timestamps in a primary workflow.
- Never download or redistribute a health dataset until the exact artifact licence, release identity, digest strategy, and storage budget are recorded.
- Never turn an association into diagnosis, treatment advice, or a health recommendation.
- Never change a hot evaluator or ABI-crossing buffer to `Vec`, `String`, or `Box`.
- Never run `cargo fmt --all`. Format only explicitly touched Rust files.
- Never use `git reset --hard`, `git checkout --`, broad deletion, or cleanup of workspace/user directories.
- Never repair unrelated failures or modify unrelated dirty files. Record them in the ledger.
- New implementation files stay below 500 lines. Files already over 1,000 lines receive no new behavior unless the packet explicitly includes decomposition.

## 5. Session size and cost controls

One session executes one task packet. A packet may be split, but packets must not be combined unless both are documentation-only.

Default limits per session:

- Read at most 12 implementation files before editing.
- Modify at most 6 implementation files plus focused tests and tracker/ledger updates.
- Add at most 700 net lines; no individual new file may reach 500 lines.
- Run targeted tests once after the coherent edit, not after every small patch.
- Run at most one broader crate check/build after targeted tests pass.
- Do not run the full workspace test suite unless the packet explicitly requests it.
- Do not browse the web unless the packet is a source/licence task.
- Do not rewrite stable working code for style consistency.
- Do not create speculative abstractions used by only one call site unless the packet requires a shared contract.

The session must stop before implementation when:

- the working tree contains overlapping user changes in a target file;
- the named backend contract does not exist;
- the task would require a new external side effect not authorized by the packet;
- an artifact's data licence is ambiguous;
- a health operation could be interpreted as medical advice;
- a security or permission decision is not specified;
- the change would require an NQuin layout or opcode modification;
- the estimated change exceeds the session limits.

## 6. Standard session protocol

### 6.1 Opening protocol

The agent should perform these actions without asking the user unless a hard stop is found:

1. Read the required files from section 3 and the assigned packet.
2. Run `git status --short` and record the baseline. Treat all existing changes as user-owned.
3. Check the ledger. If the packet is already complete, verify rather than reimplement it.
4. Search with `rg` for the actual builder, route registration, record family, daemon function, and existing tests.
5. State a plan of three to six concrete steps.

### 6.2 Implementation protocol

1. Preserve the current architecture unless the packet explicitly changes a contract.
2. Build the smallest complete user job, including loading, empty, offline, error, validation, success, and persisted reload states.
3. Separate pure projection/model code from DOM/Tauri code so logic can be unit tested natively.
4. Reuse real record/capability APIs. If unavailable, leave the action disabled with the exact prerequisite.
5. Add regression tests that would fail if the surface returned to generic CRUD or fabricated data.
6. Update the tracker conservatively only after verification.

### 6.3 Verification protocol

Use the narrowest applicable sequence:

```text
1. Unit tests for the changed pure model/service
2. Product-integrity or contract tests for the changed surface
3. cargo check/test for the changed crate or target
4. Trunk/Tauri build only when that host changed
5. Browser/native task UAT when UI changed
6. git diff --check and review only the intended diff
```

For POET UI work, normally run:

```powershell
cargo test -p poet <focused_test_filter>
cargo test -p poet --test product_integrity
$env:NO_COLOR='false'; trunk build
```

Run `trunk build` from `crates/poet`. For Webizen Desktop, use focused package tests/checks already supported by its `Cargo.toml`; do not invent a packaging or signing release step.

### 6.4 Closing protocol

Every session must:

1. Update `docs/POET_IMPLEMENTATION_SESSION_LEDGER.md` with the packet, files, tests, UAT, gaps, and next packet.
2. Report usable behavior first.
3. List tests by command and result.
4. List remaining gaps without claiming the family is complete.
5. Stop. Do not opportunistically begin the next packet.

Do not commit, push, publish, install packages, download large datasets, or send external messages unless the user explicitly asks.

## 7. Definition of done for every user-facing surface

A surface is complete only if all applicable cells are true:

| Dimension | Evidence required |
|---|---|
| User job | A named person can finish a plain-language outcome |
| Domain interaction | Appropriate board, timeline, chart, editor, mixer, graph, consent flow, or control panel |
| State | Real persisted/queryable state with identifiers and provenance |
| Actions | Create/edit/transition/revoke/export or other advertised operation works |
| Projection | Derived summaries come from real records, never canned values |
| Honesty | Loading/empty/offline/error/partial/success are distinguishable |
| Safety | Permissions, sensitivity, consent, units, and irreversible effects fail closed |
| Accessibility | Keyboard flow, labels, focus, announcements, contrast, reduced motion |
| Lifecycle | Reload and relevant checkpoint/manifold/app-host behavior verified |
| Regression | Focused test prevents generic or fabricated fallback |
| UAT | The actual task is exercised in the relevant host |

Passing compilation alone is not completion.

## 8. Programme order and review gates

Execute packets in the listed order unless a dependency says otherwise.

```text
Wave 0  Integrity and shared UI foundation
   ↓
Wave 1  POET shell simplification
   ↓
Wave 2  Person-controlled Health completion
   ↓
REVIEW GATE A
   ↓
Wave 3  Governed Q42 health assets
   ↓
Wave 4  Portable app contract and projection proof
   ↓
Wave 5  Webizen Desktop multi-app control plane
   ↓
REVIEW GATE B
   ↓
Wave 6  Remaining high-value POET domain restoration
```

At each review gate, stop implementation and ask the expert reviewer to inspect architecture, data contracts, security boundaries, visual quality, and completion claims.

## 9. Wave 0 — integrity and shared foundations

### `BASE-01` — Machine-readable surface inventory (`LUNA`)

**Outcome:** Create `docs/poet/surface-inventory.json` as the conservative source of truth for the 112 thin delegated views and already-restored workspaces.

**Read:** product-integrity remediation, master tracker, `product_integrity.rs`, route registrations, generic persistence builders.  
**Write:** inventory JSON, JSON schema or focused validator test, ledger.  
**Required fields:** stable surface ID, domain, route/builder, current implementation, intended user job, record families, capabilities, backend state, generic-delegation flag, remaining behavior, UAT state, status evidence.  
**Acceptance:** inventory count reconciles with the source audit; restored Budget and Health Overview are not marked generic; no status is more optimistic than the master tracker.  
**Verify:** focused inventory validation test plus `cargo test -p poet --test product_integrity`.

### `BASE-02` — Shared view-state and feedback primitives (`LUNA`)

**Outcome:** Reusable accessible components/helpers for loading, empty, offline, inline error, success notice, and pending mutation states.

**Read:** `native_daemon.rs`, `surface_honesty.rs`, `accessibility.rs`, Health Overview, Budget workspace, UX spec.  
**Write:** one focused shared module, CSS, two exemplar call sites, tests.  
**Acceptance:** Health and Budget use the same state semantics; status updates are announced; no global alert boxes; dynamic text remains escaped.  
**Verify:** focused unit tests, product-integrity tests, Trunk build, keyboard/DOM UAT.

### `BASE-03` — Split oversized generic persistence modules (`LUNA`)

**Outcome:** Decompose only the generic persistence files currently above repository size limits without changing behavior.

**Read:** product-integrity R0, oversized module, callers.  
**Write:** directory-backed modules separated by domain/lifecycle; imports; tests.  
**Acceptance:** no new behavior, no route changes, no delegation increase, each new file below 500 lines.  
**Verify:** existing focused tests and POET check/build.

## 10. Wave 1 — POET shell simplification

### `UX-01` — Honest shell-state audit and removal (`LUNA`)

**Outcome:** Remove or relabel fabricated global daemon, graph, job, peer, telemetry, and validation decorations.

**Read:** `topbar.rs`, `pulse_stream.rs`, `job_queue.rs`, `diagnostics.rs`, `surface_honesty.rs`, UX spec.  
**Acceptance:** every global operational value is sourced from a real state channel or explicitly labelled preview/unavailable; no deterministic fake event feed appears live.  
**Verify:** honesty regression tests and DOM inspection in daemon-offline mode.

### `UX-02` — Primary navigation and progressive disclosure (`LUNA`)

**Outcome:** Make the active manifold and current task dominant; move advanced probes, telemetry, and secondary controls behind clearly named drawers/menus.

**Read:** `topbar.rs`, `docks.rs`, `tool_widgets.rs`, `submanifold_nav.rs`, command palette, UX spec.  
**Acceptance:** common navigation remains one action away; advanced controls remain discoverable; narrow viewport works; existing keyboard shortcuts remain.  
**Verify:** focused interaction tests, Trunk build, screenshots at desktop and narrow widths.

### `UX-03` — Canvas/container visual hierarchy (`LUNA`)

**Outcome:** Standardize container chrome, status badges, spacing, typography, resize handles, selected/focused states, and calm density.

**Read:** `container_chrome.rs`, `containers.rs`, `css.rs`, `theme.rs`, Health/Budget exemplars.  
**Acceptance:** content hierarchy is readable at 100% zoom; container controls do not compete with task content; `prefers-reduced-motion` is honored.  
**Verify:** CSS/build checks and visual UAT on Research, Health, and Projects manifolds.

### `UX-04` — Modal, drawer, and form accessibility (`LUNA`)

**Outcome:** Provide focus trapping, Escape dismissal, return-focus behavior, visible labels, inline validation, and live-region feedback.

**Read:** `accessibility.rs`, modal/drawer helpers found by `rg`, UX spec.  
**Acceptance:** one shared implementation, no duplicate ad hoc traps, keyboard-only UAT passes on two representative workflows.  
**Verify:** unit/DOM tests and manual keyboard UAT.

## 11. Wave 2 — person-controlled Health

### `HLT-01` — Timeline record inspection and correction receipts (`LUNA`)

**Outcome:** A person can open a timeline item, inspect provenance, correct it without destroying the original, and see the correction relationship.

**Read:** Health spec, `overview_workspace.rs`, `model.rs`, health persistence, record API.  
**Acceptance:** append-only correction receipt; original remains queryable; timeline distinguishes corrected/current; explicit empty/offline/error states.  
**Verify:** pure model tests, record API tests if changed, product-integrity test, browser UAT.

### `HLT-02` — Vitals metric selector and accessible data view (`LUNA`)

**Outcome:** Select BP, heart rate, glucose, or supported lab metrics and inspect a real-record chart plus accessible table.

**Read:** Health spec, `overview_workspace.rs`, `model.rs`, `vitals.rs`, lab-result views.  
**Acceptance:** units never mix silently; no clinical range or interpretation appears unless sourced and explicitly authorized; keyboard selector and text/table alternative work.  
**Verify:** unit/ordering/unit-safety tests, browser UAT with empty and fixture-backed records.

### `HLT-03` — Consent grant and revocation service contract (`5.5`)

**Outcome:** Define and enforce a time-bounded, category-scoped grant and immutable revocation receipt using existing DID/delegation/deontic facilities.

**Read:** Health spec, `crdt.rs`, deontic modules, record API, existing disclosure commands in Webizen Desktop.  
**Acceptance:** immutable grant principal/scope; expiry fails closed; only authorized principal can revoke; revocation cannot reactivate; no plaintext private keys.  
**Verify:** service-level authorization, expiry, replay, and mutation tests.  
**Stop:** any NQuin layout change, key-vault operation, or absent authority rule becomes `REVIEW`.

### `HLT-04` — Consent and disclosure workspace (`LUNA`)

**Dependency:** `HLT-03`.  
**Outcome:** A person selects record categories, recipient DID/contact, purpose, expiry, reviews a plain-language summary, grants access, and revokes it in one action.

**Read:** Health spec, new consent contract, disclosure views, shared form primitives.  
**Acceptance:** no raw DID when a known-contact selector is available; explicit authority and expiry; active/expired/revoked states; receipt inspection; failed daemon/permission states.  
**Verify:** projection tests, product-integrity regression, keyboard/browser UAT.

### `HLT-05` — Conditions and medicines workspace (`LUNA`)

**Outcome:** Replace the two generic views with purpose-built lists/editors covering status, onset/start, end/resolution, dose/unit/schedule, provenance, sensitivity, and correction history.

**Read:** Health spec, `conditions.rs`, `medications.rs`, health persist model/API.  
**Acceptance:** domain-appropriate controls; active/history separation; no medication interaction claim unless backed by the real native capability and complete inputs.  
**Verify:** model validation tests, delegation ceiling reduced by two, browser UAT.

### `HLT-06` — Health documents and reports workspace (`LUNA`)

**Outcome:** Replace generic views with classified text-extract ingestion, document metadata, provenance, linked timeline entries, and an honest binary-PDF limitation.

**Read:** Health spec, backend-gaps document, `documents.rs`, `clinical_reports.rs`, existing `Document.ingest` path.  
**Acceptance:** extracted text path works; binary/scan controls remain disabled with exact prerequisite; no fake OCR/PDF parsing.  
**Verify:** ingestion projection tests, delegation ceiling reduced, browser UAT.

### `HLT-07` — Clinical calculator workflow integrity (`5.5`)

**Outcome:** Audit and rebuild calculator forms so every required input, unit, applicability condition, result provenance, and non-advice warning is explicit.

**Read:** Health spec, actual clinical engine APIs/tests, existing UI bindings.  
**Acceptance:** no defaults presented as patient values; incomplete input cannot produce a result; output names the algorithm/version and is not a diagnosis.  
**Verify:** boundary-value tests against the native implementation plus browser UAT.

### `HLT-08` — Health completion UAT pack (`LUNA`)

**Outcome:** An executable/manual task checklist covering add measurement, reload, inspect trend/table, correct record, grant access, revoke access, ingest report text, and offline recovery.

**Write:** focused UAT document/tests, tracker/ledger only; repair only defects directly found in the listed workflows.  
**Acceptance:** each result has evidence; HLT items remain `[~]` unless every requirement dimension is demonstrated.

**Review Gate A:** expert reviews the Health data/consent model, UX screenshots, browser behavior, and status claims before dataset or portable-app work proceeds.

## 12. Wave 3 — governed Q42 health assets

### `AST-01` — Q42 asset envelope and licence policy (`5.5`)

**Outcome:** A versioned core schema for upstream release identity, digests, formats, record counts, parser/mapping versions, provenance, licence/use/redistribution classes, sensitivity, validation, and bounded chunks.

**Read:** programme architecture, Qualia asset/package precedents, temporary-artifact rules.  
**Acceptance:** deterministic serialization; licence obligations propagate to derived assets; unknown licence fails closed; public API is caller-buffered where ABI-facing.  
**Verify:** round-trip, deterministic digest, obligation-union, and invalid-envelope tests.

**Result recorded 2026-09-06:** Implemented in
`crates/qualia-core-db/src/q42/asset_envelope/`. Wire format magic `Q42AST\0\0`
v1 with deterministic LE encoding; SHA-256 payload and envelope digests;
`LicencePolicy` fails closed on unknown tags; derived envelopes inherit the
most-restrictive obligation union; chunk plans reject budgets above 42 MiB.
`cargo test -p qualia-core-db --lib asset_envelope` → 11 passed.

### `AST-02` — Bounded import job framework (`5.5`)

**Outcome:** Cold-construction import jobs use unique `TempDir`, explicit byte/record budgets, streaming chunks, cancellation, quarantine counts, and promote-on-success semantics.

**Acceptance:** no broad temp cleanup; success/error/unwind cleanup tests; 42 MB pass budget; raw artifact immutable; no network downloader required.  
**Verify:** budget, cleanup, cancellation, partial-input, and deterministic-output tests.

### `AST-03` — ChEBI release parser (`5.5`)

**Dependency:** `AST-01`, `AST-02`.  
**Outcome:** Parse one documented ChEBI bulk format from a caller-selected local file into normalized records and quarantined errors.

**Constraints:** use tiny synthetic format fixtures; do not download the release; preserve IDs, synonyms, parent relationships, source release, and attribution; do not add runtime URI strings to hot paths.  
**Verify:** malformed row, oversize, cancellation, deterministic mapping, and count reconciliation tests.

### `AST-04` — ChEBI Quin mapping and validation (`5.5`)

**Outcome:** Map accepted ChEBI records into evidence-preserving Quins plus resolver/lexicon entries and a validation report.

**Acceptance:** q_hash/60-bit tag rules followed; parity valid; original source record address retained; no silent identifier collision; caller-buffered hot projection.  
**Verify:** zero-allocation measurement where Tier 1 applies, parity, deterministic mapping, SHACL/shape validation.

### `AST-05` — Chemical knowledge query capabilities (`LUNA`)

**Outcome:** Implement bounded `describe_release`, `resolve_chemical`, relationship lookup, evidence lookup, and subgraph export over the imported asset.

**Acceptance:** every result carries asset version, source/evidence, uncertainty where present, and licence obligations; caller buffers enforce limits.  
**Verify:** empty/ambiguous/limit/cross-reference tests.

### `AST-06` — POET food/compound evidence explorer (`LUNA`)

**Outcome:** A calm search → entity → relationships → evidence/provenance workflow in the Health manifold.

**Acceptance:** associations are labelled research evidence; no recommendations; source, release, evidence and licence drawer visible; offline/no-asset state explains how to install/import an asset.  
**Verify:** safe rendering tests, browser keyboard UAT, no demo claims.

### `AST-07` — Source catalogue without data bundling (`LUNA`)

**Outcome:** Register FooDB, HMDB, CTD, Monarch, ABCkb, FoodAtlas, Phenol-Explorer, FOODBALL, PhInd, and Cytoscape as source/connector descriptors with conservative acquisition status.

**Acceptance:** descriptors contain official URL and `unverified/restricted/connector/catalogue` state; no dataset is downloaded or marked redistributable without verified artifact terms.  
**Verify:** schema validation and unknown-licence fail-closed tests.

Later importers are one source per new `5.5` packet. Start Monarch only after source-partitioned licence propagation is designed. FooDB, HMDB, CTD, Phenol-Explorer, ABCkb, FoodAtlas, and PhInd require artifact-specific licence/access review before any bundling. Cytoscape is a connector/export target, not an ingest dataset.

## 13. Wave 4 — portable app contract

### `APP-01` — Manifest/type reconciliation ADR (`5.5`)

**Outcome:** Document and test the relationship among POET `Manifest`, `ManifoldSeed`, `ConstructPackage`, HCF/HMC envelopes, Webizen qApp host/export, and the new portable application contract.

**Acceptance:** no duplicate competing manifest is implemented before the ADR; legacy `qApp` naming is mapped or deprecated explicitly; POET is not special-cased in the final host model.

### `APP-02` — Portable application manifest v1 (`5.5`)

**Outcome:** Define a versioned manifest containing app identity/version/author, entry projections, required capabilities/assets, state schema, permission intents, presentation hints, compatibility, integrity, and update channel.

**Acceptance:** deterministic serialization; unknown versions/permissions fail closed; no executable path traversal; presentation hints cannot grant authority.  
**Verify:** round-trip, malformed, unknown-version, permission-escalation, and deterministic-hash tests.

### `APP-03` — Projection adapters (`5.5`)

**Outcome:** Resolve one manifest into a POET manifold, POET container, focused mini-app entry, and Webizen Desktop launch descriptor without duplicating state or permissions.

**Acceptance:** stable app/capability/asset IDs across projections; same authorization result; no projection-specific private database.  
**Verify:** projection conformance matrix tests.

### `APP-04` — Health app proof package (`LUNA`)

**Dependency:** `APP-02`, `APP-03`, Health Wave 2.  
**Outcome:** Package the Health experience with at least one manifold entry, one measurement/evidence container entry, and one focused entry using the same manifest.

**Acceptance:** no copied health algorithms or records; offline/permission states match across projections.  
**Verify:** package round-trip and cross-projection state tests.

### `APP-05` — Focused mini-app shell (`LUNA`)

**Outcome:** Implement the smallest generic focused-app renderer/host necessary to run the Health proof entry.

**Acceptance:** renderer is manifest-driven, accessible, and does not contain health-specific storage/business logic; unsupported presentation hints fail safely.  
**Verify:** Health proof UAT and a second minimal non-health fixture.

### `APP-06` — App conformance suite (`LUNA`)

**Outcome:** Automated tests check manifest integrity, capability resolution, asset presence, permission decisions, state identity, projection availability, and honest unsupported states.

**Acceptance:** deliberately inconsistent projection fixtures fail.

## 14. Wave 5 — Webizen Desktop control plane

### `WD-01` — Control-plane information architecture (`LUNA`)

**Outcome:** Replace POET/qApp-centric navigation with **Apps, Node, Identity & Permissions, Assets, Connections, Recovery**.

**Read:** Desktop spec, programme architecture, current shell/static portal/menu files.  
**Acceptance:** POET appears under Apps; domain logic does not move into Desktop; old routes remain reachable during migration; no static fake daemon status.  
**Verify:** shell/menu tests and visual UAT.

### `WD-02` — Installed-app registry and inspection (`5.5`)

**Outcome:** Enumerate bundled/installed manifests, verify integrity/compatibility, and expose read-only app details and permission intents.

**Acceptance:** bounded storage; malformed package quarantined; registry does not execute an app while inspecting it; POET is the first bundled manifest.  
**Verify:** registry/quarantine/integrity tests.

### `WD-03` — App lifecycle supervisor (`5.5`)

**Outcome:** Install, launch, stop, update, and uninstall app packages through explicit lifecycle states and receipts.

**Acceptance:** no arbitrary shell command from a manifest; launch is allowlisted; uninstall targets exact managed directories and is recoverable where practical; running state is real.  
**Verify:** state-machine, path traversal, failed launch, rollback, and receipt tests.  
**Stop:** signing/key-vault design is `REVIEW`.

### `WD-04` — Node and daemon workspace (`LUNA`)

**Outcome:** Real daemon health/start/stop/restart/log status plus CPU/RAM/GPU/Sentinel/thermal data only where real providers exist.

**Acceptance:** unsupported telemetry is absent or unavailable, never simulated; process transitions cannot double-start; log view bounded and redacted.  
**Verify:** supervisor tests and offline/running/crashed UI UAT.

### `WD-05` — Identity and app-permission workspace (`5.5`)

**Outcome:** Inspect app permission intents, existing grants, denials, expiry, and revocation without exposing private key material.

**Acceptance:** default deny; grant bound to app ID/version/principal/scope; presentation cannot bypass policy; key import/export remains out of scope unless separately reviewed.  
**Verify:** authority, escalation, expiry, and revocation tests.

### `WD-06` — Asset manager workspace (`LUNA`)

**Outcome:** Inspect Q42 assets, versions, sizes, digests, licence obligations, validation/quarantine state, dependent apps, and safe removal eligibility.

**Acceptance:** cannot label unknown licence as unrestricted; cannot remove an in-use asset without an explicit dependency decision; no automatic remote download.  
**Verify:** projection tests and visual UAT.

### `WD-07` — Connections and Recovery workspaces (`LUNA`)

**Outcome:** Real connector/peer transport status, failed-job history, quarantined imports, backups/migrations already supported by the backend, and auditable retry actions.

**Acceptance:** Pure operations may use bounded retry; unknown/side-effecting operations require a new explicit run; unavailable transports remain honest.  
**Verify:** retry-policy tests and offline UAT.

### `WD-08` — POET plus Health app hosting proof (`LUNA`)

**Outcome:** Webizen Desktop launches/stops POET and the focused Health app through the same registry/lifecycle/permission system.

**Acceptance:** no POET-only launch branch; both apps show real lifecycle and permission state; authorization/data results match POET projection tests.  
**Verify:** end-to-end desktop UAT and app conformance suite.

**Review Gate B:** expert reviews package security, permission enforcement, managed paths, process lifecycle, UI architecture, and cross-projection parity before broader restoration.

## 15. Wave 6 — remaining high-value POET restoration

These packets reduce the generic delegation count while restoring domain jobs. Execute in order; each is one or more sessions, never one broad rewrite.

| Packet | Model | User outcome | Minimum acceptance |
|---|---|---|---|
| `PRJ-01` | `LUNA` | Create/edit/move tasks on a real Kanban board | Persisted transitions, validation, keyboard movement, delegation count reduced |
| `PRJ-02` | `LUNA` | Define dependencies and inspect blockers/timeline | Cycle detection, real derived blockers, accessible alternative to graph |
| `PRJ-03` | `LUNA` | Track risks and accept/reject deliverables | Evidence, authority, status history, no false settlement/completion |
| `GOV-01` | `5.5` | Author agreement parties, clauses, scopes, obligations | Real deontic mapping, expiry/defeaters, immutable principals |
| `GOV-02` | `5.5` | File dispute, add evidence, record decisions/remedies | Append-only evidence/history, authority gates, no legal-effect overclaim |
| `KNOW-01` | `LUNA` | Inspect dataset lineage and map fields visually | Provenance, lossy-mapping warning, validation navigation |
| `KNOW-02` | `LUNA` | Explore/edit ontology relations and SHACL violations | Real graph state, safe labels, accessible table/tree fallback |
| `STU-01` | `LUNA` | Mix real audio session channels and use transport | Real backend session, honest unavailable device state, no decorative meters |
| `STU-02` | `LUNA` | Edit scene hierarchy, transforms, materials, lighting | Real scene state, undo/checkpoint, no fake GPU output |
| `DEV-01` | `5.5` | Inspect devices, grant roles, and initiate permitted sessions | Explicit permission session, no claimed remote control without transport |
| `SOC-01` | `5.5` | Attach Library assets and enforce blocked relationships | Immutable receipts, blocked submission rejected at backend boundary |

For each packet, read its canonical domain spec and inventory entry, create a pure model/service layer, implement the domain interaction, add a product-integrity regression, run task UAT, reduce the delegation ceiling by the number of genuinely restored thin files, and leave the tracker conservative.

## 16. Prompt templates

### 16.1 First Luna session

```text
Work in C:\Projects\qualia-27062026. Execute only packet BASE-01 from
docs/POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md.

Read AGENTS.md in full, then sections 1-8 and BASE-01 of the playbook, then
docs/POET_IMPLEMENTATION_SESSION_LEDGER.md. Follow the session limits and stop
rules exactly. Treat all existing changes as user-owned. Do not combine tasks,
do not browse, do not commit, and do not increase any completion or delegation
ceiling. Implement, run the packet's targeted verification, update the ledger,
and end with usable outcome, changed files, test results, remaining gaps, and
the recommended next packet.
```

### 16.2 Subsequent session

```text
Work in C:\Projects\qualia-27062026. Continue the programme by executing only
packet <PACKET-ID> from
docs/POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md.

Read AGENTS.md in full, sections 1-8 plus <PACKET-ID>, and the session ledger.
Verify that dependencies are complete before editing. Preserve unrelated dirty
work. Follow the packet read/write scope, acceptance criteria, tests, retry
limit, and stop rules. Do not start the next packet. Update the ledger and give
a concise evidence-based handoff.
```

### 16.3 GPT-5.5 packet

```text
Execute only packet <PACKET-ID> from the lower-cost execution playbook. This is
a GPT-5.5 boundary task: prioritize contract correctness and existing Qualia
invariants over breadth. Read AGENTS.md, sections 1-8, the packet, its named
canonical specification, and the ledger. Do not infer missing authority,
licensing, ABI, clinical, or security rules. Stop and record a review blocker
if one is absent. Keep the change session-sized, add negative-path tests, run
targeted verification once, update the ledger, and do not begin another packet.
```

### 16.4 Expert review request after a gate

```text
Review the completed work through Review Gate <A-or-B>. Do not begin new feature
work. Read AGENTS.md, the lower-cost execution playbook, the session ledger,
the relevant canonical specs, and every diff attributed to packets in this
gate. Audit architecture, authorization, data/provenance, health or package
safety, product-integrity claims, accessibility, visual quality, tests, and
cross-projection behavior. Fix only clear in-scope defects; otherwise produce
a prioritized correction list with exact files and evidence.
```

## 17. Final handoff expected before expert review

The lower-cost implementation run should finish with:

- a ledger with no ambiguous packet states;
- a list of all modified and new files by packet;
- test/build/UAT evidence and known environmental failures;
- before/after delegation count;
- screenshots for major UI packets;
- unresolved security, licence, clinical, ABI, or architecture questions;
- tracker changes separated from claims still awaiting expert approval;
- no orphaned development servers, temp artifacts, downloaded bulk datasets, or broad uncommitted formatting changes.

