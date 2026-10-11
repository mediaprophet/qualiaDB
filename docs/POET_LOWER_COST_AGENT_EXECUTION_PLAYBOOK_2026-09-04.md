# POET / Webizen Lower-Cost Agent Execution Playbook

**Current entry point for Storytold / OS-shell work:** [section 18: ST programme](#18-st-programme-execution-packets-2026-10-11), governed by [the current implementation specification](work-in-progress/POET_UPDATED_IMPLEMENTATION_PLAN_2026-09-05.md#st-programme-2026-10-11--storytold-apps-poet-and-webizen-os). Start with `ST-00`, not the historical September starting prompt. Section 18 corrects checkout paths, target repositories, UI ownership and verification for this programme. Earlier packets and historical evidence remain intact.

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

---

## 18. ST programme execution packets (2026-10-11)

### 18.1 Read this before selecting a packet

The [ST implementation specification](work-in-progress/POET_UPDATED_IMPLEMENTATION_PLAN_2026-09-05.md#st-programme-2026-10-11--storytold-apps-poet-and-webizen-os) is the architecture and contract authority for these packets. ST-A through ST-M are its stable section labels. This section is the executable queue, not a claim that the named future crates/files already exist. Source references below are relative to the explicitly named repository, not the old monorepo checkout. Crate-qualified shorthand such as `vibe/src/` or `qualia-core-db/src/` means `C:\github\qualiaDB\crates\<crate>\src\`; `core q42/` or `core render/` means that subdirectory of `qualia-core-db/src/`. Poet `wasm/` means `C:\github\poet\wasm\`, not the retained monorepo copy. Dependencies refer to ST IDs only unless expressly stated. Historical APP/WD evidence may be reused after checking the live implementation; never reimplement valid work merely to tick a new packet.

**Routing:** D1 mechanical inventory, D2 bounded implementation, D3 cross-module integration, D4 authority/storage/ABI/executable-code boundary with independent review, D5 principal-only decisions. Use a proven inexpensive coding agent for D1/D2; use a stronger reviewer for D3 and an independent security/systems reviewer for D4. No vendor/model name conveys permission. Low-cost agents do not invent security policy, package ABI, numeric-oracle tolerances, external budgets or completion scores.

**Correct roots:** `C:\github\qualiaDB`, `C:\github\poet`, `C:\github\webizen-os-shell`, `C:\github\webizen-desktop` and source `C:\github\storytold`. QualiaDB and Poet planning changes are on `0.0.40.8-dev`. Other repositories have not been switched by this planning session. Before modifying a different repository, inspect status/branch, preserve dirty work, and create/switch the matching development branch only within the principal's branch authorisation. No worktrees, extra engine clones, wholesale deletion, implicit commits or pushes.

**Session contract:** one packet subunit, at most six implementation files plus focused tests/ledger, new implementation files under 500 lines, at most 700 net lines. Split packet subunits before exceeding this. The parent packet closes only after all subunits pass; a split is not feature abandonment. Read governing rules plus ST-C/E/F/G/H as applicable and assigned source seams, not every domain. Check baseline/overlap/claims, write failing tests first where infrastructure exists, implement, run focused tests, one relevant broader check, record evidence, stop. After two materially different failed attempts report the exact blocker and preserve work; environment/auth/permission failures are not authority to change project security controls.

**Claims and permissions:** planning complete is not implementation complete. Do not mark complete with zero tests, skipped UAT, fake live data, swallowed error, unimplemented button, compiler-only proof, unexplained lossy format mapping, or an unsupported label covering a new regression. No external API calls, paid jobs, model downloads, public remotes, signing secrets, user-data deletion, release publishing or production queries occur in a normal packet. Use synthetic doubles. Show the principal exactly which out-of-band decision is needed while continuing independent local work.

### 18.2 Mandatory evidence and handoff

Append one row per attempt to `C:\github\qualiaDB\docs\POET_IMPLEMENTATION_SESSION_LEDGER.md` without rewriting previous rows. Include packet/subunit, actor/model, all repository HEADs and relevant dirty state, branch, files, schema/API changes, exact test/build commands and results, actual test count or 'not run', local/native/held paths, artifact paths, remaining rows, blockers and next packet. New execution evidence is retained in a bounded caller-selected artifact directory; use owned RAII scratch and promote only validated outputs. Never create unscoped root logs or screenshots of unrelated windows.

Packet states: NotStarted, InProgress, Partial, Blocked, ReviewReady, Complete. Complete requires listed acceptance, real checks and required review. Gate review is a separate ledger entry. The author cannot self-close G1/G2/G4. The coordinator assigns one write lane; parallel agents own disjoint app/primitive files. Shared Cargo/module registrar files have one integrator, not concurrent writers.

### 18.3 ST-00 — Source, build and dependency baseline

**D3 / COORD, QA. Dependencies:** none. **Write:** existing planning/ledger files only; no product change. **Read:** root Cargo files in all four main repos; Poet `wasm/tests/{product_integrity,surface_inventory}.rs`; OS-shell `src/{app,state/shell_state,components/stage_host}.rs`; Desktop `src/lib.rs`; all eleven app root Cargo/AGENTS; architecture/format-gap docs for the ten Rust craft apps; ArtCraft root AGENTS, root Cargo.toml and frontend/package.json instead of nonexistent standard architecture/gap docs.

Subunits: (a) record actual HEADs, paths, dirty files, executable entries, Rust/wasm-bindgen/Trunk/Dioxus versions and installed targets; (b) inspect dependency/features using `cargo metadata --no-deps --format-version 1` and `cargo tree -e features` per consumer; record shell's Dioxus patch revision versus other consumers and choose one existing pinned compatible dependency source for ST-10, without upgrading; (c) locate actual source/test seams and baseline focused checks from ST-L, keeping failures separate; (d) inventory commands, UI-only handlers, gestures, formats, payloads, device hooks and real status providers for each app.

Coverage-row schema: source repo/revision/file/symbol, feature/command/gesture ID, read/mutation/external effect, strict-parameter-schema state, target/selection dependencies, native/WASM support, UI destination, Qualia primitive owner or promotion need, fidelity limitation, licence/provenance, synthetic fixture, required assertion, packet/subunit, evidence/state. Use actual registered IDs, not README counts. Inventory generation must include UI-only tables and pointer tools, not only CommandSpec names. Keep generated coverage in the corresponding existing programme/ledger until ST-01 establishes owned app resources; dedicated machine-readable inventory resources are then permitted product/test inputs.

**Acceptance:** all eleven apps present; retained monorepo Poet copies labelled noncanonical; native CLI not confused with graphical UI; StageHost/peer placeholders identified; Dioxus webview fact recorded; source deficits and build blockers have exact evidence. Do not run all suites just to baseline. **Next:** ST-01 and ST-02.

### 18.4 ST-01 — Local app forks and licence/asset baseline

**D2 / OPS, QA; D5 for ambiguous rights. Depends:** ST-00. **Source:** eleven `<slug>-main` directories. **Destination:** local `C:\github\poet-<slug>` default from ST-C4, one at a time.

Verify parent and existing destination before creating anything. If destination exists, inspect it rather than overwrite. If upstream Git history is available, preserve it without rewriting; if snapshot lacks history, record source archive identity/digest, do not invent ancestry or contact a guessed remote. Create local repository/workspace only; public owner/name/publishing stays unapproved. Copy attributed code/assets, licence/NOTICE files and lockfiles; preserve source snapshots. Record optional craft-fonts/corpus dependencies, vendored patches, CAD MPL dependency, brand trademark restrictions and ArtCraft's absent services repository. Use generic local names/icons until final branding; no proprietary reference assets.

**Tests/gate:** metadata resolves in the local fork without relying on `<slug>-main`; source baseline fixtures remain available; optional font absence does not fail portability; provenance/attribution inventory covers shipped assets. Unknown rights block shipping that asset, not unrelated adapters. **Next:** each app's ST-19 adapter after G1/hosting.

### 18.5 ST-02 — Contract crate and G1 review

**D4 / SPEC, RUST, SEC. Depends:** ST-00. **Read:** ST-D/E/G/H; core `q42/app_manifest/`, `q42/app_registry/`, existing `poet_host/invoke/coverage.rs`; Poet `wasm/src/tool_chest/core/intent_bus.rs`; sibling manifests. **Write:** planned `qualia-app-contract` directory-backed modules and workspace registrar, or one reviewed existing contract owner; no app/frontend rewrite.

Subunits: schema/identity/decimal-u64 validators; descriptors/targets/effects; request/receipt/session/job DTOs; surface/input/gesture DTOs; canonical codec/version tests; required application-resource/catalogue mapping to existing manifest. Fixed-capacity compiled tables are distinct from cold serde DTOs; preserve exact original IDs and reject hash collisions rather than merge. Permission declarations do not become grants. Define actual transport entry/resources and replay rules without widening frozen Vibe Host. If a new capability binding is necessary, record its exact name, function, discovery row, validation and WASM profile in the G1 review; unreviewed bindings stay unavailable.

**Tests:** reject empty/duplicate command IDs, unknown version/enum/effect, invalid decimal u64, JS-unsafe numeric conversion, malformed schema/params, duplicate aliases, unsupported target, nonfinite coordinates and oversized payload. Deterministic encode/decode preserves target order/versions. Compatibility fixture reads existing PortableAppManifest without a new ABI. Contract-only dependency tree has no Dioxus/Tauri/egui/wgpu/CUDA/daemon.

**G1:** independent reviewer confirms ST-E interfaces, allowed effects, state ownership, runtime entry/isolation contract, manifest reconciliation and narrow features. Document exact accepted interface version and affected files. Other code can prepare fixtures while blocked; executable adapters cannot guess missing authority.

### 18.6 ST-03 — Extract service supervisor without Tauri

**D2 / RUST, QA. Depends:** G1. **Read:** Desktop `src/supervisor.rs`, callers in `src/main.rs`; OS-shell Cargo. **Write:** planned `crates/webizen-host/src/supervisor/` in OS-shell repo plus minimal Desktop compatibility wrapper.

Move service/operation states, snapshots, bounded history and completion/error reporting behind toolkit-free interfaces. Parameterise clock/event sink; no AppHandle, State, Emitter or global UI imports. Preserve existing state names/wire compatibility. Desktop wrapper delegates rather than keeping a second state map. Shutdown/pause/restart semantics remain explicit; this service does not mint agent authority.

**Tests:** starting/ready/degraded/failed/stopped; duplicate start; completed-operation retention bounded; dropped handle/error reports; subscriber disconnect and shutdown. New host lib builds with default features off and dependency tree excludes Tauri/Dioxus. Legacy adapter focused tests still pass. **Next:** ST-04/05.

### 18.7 ST-04 — Extract engine/session/daemon services

**D3 / RUST, QA. Depends:** ST-03. **Read:** Desktop `src/commands/{poet,vibe_host,poet_daemon,q42}.rs`, `src/{runtime,companion_gateway,telemetry_bridge}.rs`; core service/invoke APIs. **Write:** host `engine/`, `gateway/`, `telemetry/` plus compatibility wrappers; integrate one service per subunit.

Separate toolkit commands from services; inject the existing graph/Vibe execution context, authorised store and bounded event transport. Do not duplicate a snapshot-private graph when an authoritative session exists. Keep legacy request/result compatibility at adapters, not in core semantics. Telemetry distinguishes unavailable/stale/local/live; no default DID, peers or mixnet-active status. No native daemon requirement in contract crate/browser-local operation. Frozen four-op facade remains unchanged.

**Tests:** same request through old wrapper/new service yields same revision/result/diagnostic; daemon absent/down does not become success; two clients share intended session identity; invalid request never mutates; telemetry disconnected/stale states; bounded event overflow reports loss. **Next:** OS-shell integration in ST-10.

### 18.8 ST-05 — Platform and surface adapters

**D3 / RUST, QA; D4 for device/file/key authority. Depends:** ST-03, G1. **Read:** Desktop `src/{native_surface,mic_capture}.rs`, core `render/gpu/portal_gpu/{constructors,settings}.rs`, existing host file/capture entry points. **Write:** host `platform/`, `surface/`, `capture/`; touched wrappers only.

Separate window handle ownership from canonical rendering. Port child-surface lifecycle, resize/DPI/focus/pointer forwarding and release; no HWND assumption on non-Windows. File/clipboard/capture providers operate on validated scoped handles. Browser equivalents are explicit capability implementations; unavailable hardware is reported, never simulated. Key provider is an opaque provisioned service only; no key export or new cryptographic scheme.

**Tests:** create/resize/destroy twice; device lost; DPI conversion; capture denial/revocation; cancellation releases device; handle scope/path traversal; no key material in diagnostics; unsupported OS returns typed unavailable. Native surface UAT plus browser fallback assertion, narrow renderer test (hardware absence recorded). **Next:** loaders and graphics UI.

### 18.9 ST-06 — Canonical package lifecycle

**D4 / RUST, SEC. Depends:** G1, ST-03, ST-04. **Read:** core portable manifest/registry; client-core `qapp_install.rs`, `qapp_paths.rs`, `qapp_version.rs`. **Write:** core lifecycle adaptation and host package service; preserve old qapp compatibility entry.

Implement ST-D states and managed staging/activation/rollback against canonical manifest identity. Reuse verified old hashing/signature/path code where valid; inspect cannot load code. Catalogue/resources have digests and package-relative paths. Validate compatibility, entry allowlist, assets, permissions and schema before launch. Production signed-package policy requires provisioned trust; local development fixtures are labelled unsigned and cannot silently become trusted. Scope permission grants to app/version/principal/target/effect; v1 kind-only grant checking is not sufficient enforcement of scoped runtime actions. Unknown permission kind is denied; any codec change is versioned/reviewed.

**Tests:** path traversal/symlink escape where applicable; malformed/incompatible/digest mismatch quarantines; unsigned production denied; wrong signer denied; permission missing; registry capacity; interrupted activation; rollback retains old version; uninstall leaves documents/assets; revocation stops future dispatch. Use managed temp fixtures, never uninstall real packages during tests. **Next:** ST-07/08; independent review before external loading.

### 18.10 ST-07 — Browser-local worker execution

**D3 / RUST, FE, QA; independent D4 isolation review. Depends:** ST-01/06, G1. **Write:** Poet shared worker client and fork reference `apps/poet-wasm`; host/browser transport adapter. **Read:** current Poet `wasm/src/browser/capabilities.rs`, app web services, accepted contract.

Load only validated package entries in dedicated workers, exchange versioned CBOR messages and transfer bounded binary payloads. Session/request IDs and cancellation are explicit. Enforce worker admission/queue limits, CSP/origin and capability-granted host calls; package code never receives an unrestricted engine/global object. Runtime budgets use accepted host profiles, fail closed when unenforceable. Do not equate WebAssembly with complete OS isolation. No hidden HTTP/native dependency; a browser lacking workers uses an approved bounded cooperative fallback or honest unavailable, not unrestricted main-thread heavy execution.

**Tests:** malformed/oversize worker reply, unsupported export/version, wrong session ID, denied capability, worker crash/restart, timeout and cancelled request, queue overflow, absent daemon, late response cannot mutate wrong revision. Browser smoke opens reference app offline and saves/reopens via ST-09. **Next:** ST-19.

### 18.11 ST-08 — Native managed worker execution

**D4 / RUST, SEC, QA. Depends:** ST-01/05/06. **Write:** host `worker/native/` and reference `apps/poet-worker`, private IPC dispatcher. No native Rust dylib loading.

Use versioned length-delimited CBOR over a private host-created pipe/stream. Restrict executable path to validated managed entries; no shell command interpolation from manifest. Host maps authorised payload handles, checks command target/grant at each request and imposes platform process/resource policies. Worker gets minimal provisioned capabilities; record any platform sandbox unavailable and prohibit untrusted-code launch there. Handle worker crash, shutdown, in-flight mutation and restart through durable revision checks. No private keys in worker protocol.

**Tests:** truncated/oversize/frame-flood IPC, executable escape, wrong identity/version, resource exhaustion, concurrent worker sessions, crash before/after commit, process termination and restart, denied transport request, expired/revoked grant. Native reference-app UAT proves a real child process and no dependency on legacy Desktop executable. **Next:** ST-19; requires independent security review.

### 18.12 ST-09 — Domain payload plus semantic transaction durability

**D4 / RUST, SEC, QA. Depends:** G1, ST-04, accepted loader/session interfaces. **Read:** core `q42/journal.rs`, `q42/volume/`, asset envelope, Poet `wasm/src/browser/manifest.rs`, app recovery code. **Write:** core `q42/app_state/` planned modules and host/browser storage adapters.

Implement ST-H prepared/committed transaction and recovery semantics with payload references. Define exact versioned record codec before implementing; preserve existing Q42 data, ABI and privacy. OPFS is durable browser store; localStorage is preference/legacy import only. Add migration preflight/backups and actor attribution; mark dirty/saved correctly. Checkpoint binds domain and semantic revisions, linked assets and schema/engine/process digests. Undo changes document state; navigation Back only changes view. Retain encrypted/signed-file constraints and optional external assets.

**Tests:** failure injection before payload staging, after prepare, before/after commit, graph publish and ack; reopening sees either old or complete new state, never mixed domain/graph; duplicate request idempotent; stale revision refuses; quota/disk full; torn append; unknown schema; missing asset; actor wrong; cancellation cleans only owned staging; old checkpoint imports; older processing version retained. Browser/native reopen parity requires actual file/OPFS I/O. **Next:** all apps, ST-36.

### 18.13 ST-10 — Shared Dioxus UI and real OS-shell mount

**D3 / FE, RUST, UX. Depends:** G1, ST-03/04/05; loader/durability dependencies for live app mount. **Read:** shell `src/{app,components/stage_host,state/shell_state}.rs`, root Dioxus patches; Poet wasm entry; Studio Poet components. **Write:** planned `crates/webizen-ui`, root workspace wiring, shell stage mount, Poet composition integration in focused subunits.

One accepted pinned Dioxus source/version for presentation consumers; preserve repository security controls. Extract tokens and accessible primitives first. Pure UI-model tests do not depend on renderer. Replace StageHost text with real lifecycle/data-driven mount; remove fixed operational status and use actual service snapshots. Create graphical native Poet projection by mounting the same shared authoring components in the shell; do not rename the CLI into a GUI. Migrate current browser responsibilities incrementally with an explicit route/feature transition, preserving old UI as fallback until equivalents pass. No new parallel full-feature Poet implementation and no mandatory Tauri/egui/React in canonical presentation.

**Tests:** mount/start/stop/failure/unavailable states, real status provider disconnected, no hardcoded principal, no duplicate window ownership; theme/control accessibility, narrow viewport, focus return; browser/native equivalent layout and action IDs. Cargo native/browser dependency graphs use correct target gates and engine features. **Next:** ST-11/12.

### 18.14 ST-11 — Unified command registry and dispatch

**D2 / RUST, QA after G1; D3 for integration. Depends:** ST-02/10. Implement ordinary principal-authorised dispatch against G1; delegated-agent mutation remains disabled until ST-17/G2. G2 is not a prerequisite for building this registry, because ST-17 consumes it. **Read:** Poet tool/intent/registration and command palette tables; ST-E. **Write:** UI-model `commands/` and Poet adapter; no new authority policy.

Register descriptors once, deterministic merge of app/shared aliases, namespace and conflict reports. Menu/toolchest/orb/palette/shortcuts/agent discovery refer to descriptor IDs. Dispatcher checks the accepted execution boundary, not labels/onclick-private mutations. Existing actions migrate behind adapters; unavailable ones keep a reason. Preserve command grouping versus executable workflow distinction. Do not bulk append to 4,000-line action routers; create focused routes and stable reexports.

**Tests:** collision rejects, all surfaces resolve same IDs, aliases choose only compatible target, checked/disabled state updates, schema errors propagated, missing binding honest, agent/UI request reaches same handler, zero-test filter rejected by handoff. **Next:** ST-12/13/14.

### 18.15 ST-12 — Context and precise hit-target resolver

**D2 / RUST, FE, QA. Depends:** ST-11, ST-05 for native targets. **Read:** Poet radial_gesture, specialised surface input, core semantic picking; ST-F1. **Write:** UI-model `context/` and per-surface input adapters.

Implement priority order, pointer versus focus context, selection retention and explicit retarget. Tag pick requests with surface/frame/camera revision. Do not silently change multi-selection on inside hit or lose text selection when moving focus to menu. Translate native/browser DPR coordinates once. Target objects must resolve through domain IDs/semantic map; header/background targets are separate.

**Tests:** text span, selected cell range, image layer, clip, CAD entity, container header, empty canvas, active modal; right-click another document while previous focus stays; group inside/outside rules; overlapping picks; late GPU readback; destroyed/moved object; NaN/DPR; context remains stable through menu opening and is revalidated on execute. **Next:** menu/orb.

### 18.16 ST-13 — Contextual menubar, toolchest and palette

**D2 / FE, UX, QA. Depends:** ST-11/12. **Write:** shared UI `menu/`, `toolchest/`, `palette/`, native menu adapter and Poet bridges; each is a separate subunit. **Read:** old Poet `topbar/menu.rs`, palette entries and toolbox metadata, ST-F2.

Render stable shell menus plus document-domain contributions. Focus, not hover, changes domain menus. Target indicator and explicit SaveDocument/Checkpoint/Export/CloseView. Checked/radio/disabled reasons and native/macOS projection use one model. Shortcut scope follows ST-F2; IME/typing not intercepted. Search includes unavailable commands with reasons, no label dispatch. Parameter panels use validated schema and user-friendly widgets.

**Tests:** switching doc/sheet/clip context; title/Undo action; save distinction; duplicate shortcuts; keyboard-only navigation/escape/focus restore; screen-reader menu semantics; installed package removal refresh; same command handler via all presentations; dynamic/user labels safely escaped. **Next:** ST-14/15.

### 18.17 ST-14 — Orb graph model and navigation

**D2 / RUST, UX, QA. Depends:** ST-11/12. **Write:** shared UI `orb/{model,layout,navigation}.rs` and pure tests. **Read:** ST-F3 and old radial menu sectors as compatibility inventory, not final design.

Implement root/context target, group expansion into connected child, tagged edges, explicit retarget, pin/back/collapse/dismiss. Maximum six-depth path and deterministic bounded visible set; root shows at most eight entries plus More. No adaptive ordering. Expansion and pinning never invoke a command or grant. Closing view differs from deleting content. Accessible tree model shares IDs/state with spatial view.

**Tests:** inherited context; retarget edge; cycle/depth rejection; deterministic positions/ordering; viewport corners; root replacement; pin unpin cleanup; back does not undo; stale/deleted target disabled; expansion emits no execution; More includes every remaining applicable command. **Next:** ST-15/16.

### 18.18 ST-15 — Orb rendering with upgraded graphics

**D2 / FE, UX after accepted render seam; D3 / RUST for adapter. Depends:** ST-05/10/14. **Read:** core `render/{hud,scene_graph,navigation}`, `render/gpu/portal_gpu/{picking,settings,render_frame}`; renderer SDK. **Write:** shared UI `orb/{view,input,graphics_adapter}` and narrowly scoped core adapter only if necessary.

Draw orb bodies/connections with canonical graphics, screen-space readable labels/controls, fallback tree/2D orb. Use existing material/lighting/camera capabilities only when available; no new draw engine/device. Right-click/long-press/context-key same root; cancel touch timer on drag; primary outside dismiss; secondary click never immediately dismisses. Pointer capture/hit areas remain stable. Keyboard focus and approval controls remain available if GPU fails. No labels hidden behind sphere or bloom/temporal effects.

**Tests/UAT:** all four viewport edges, zoom/DPR, nested expansion, occlusion, resize, device loss, reduced motion, touch/pen timer, keyboard/tree equivalence, long submenu names, blocked command appearance. Native/browser screenshots of app window only; real graphics receipt if available, not mock 'GPU active'. Allocation/resource evidence respects ST-I. **Next:** ST-16.

### 18.19 ST-16 — Macroscopic/microscopic attention

**D2 / UX, FE, QA; D3 camera adapter. Depends:** ST-12/14/15. **Write:** shared UI `attention/`, Poet composition bridge, camera-target adapter. **Read:** Construct model and ST-F4/F5.

Implement AttentionFrame push/pop with object bounds-based focus, breadcrumb, selection/view/camera snapshot and overview backdrop. Right-click does not fly camera; Focus/Explore does. Parent Construct and object identity persist. Object subparts and parameter orbs add semantic depth without inventing global app/scene kinds. Preserve edits on Back; offer separate document Undo. Reduced-motion skips camera tween. Geometric LOD responds to render policy, not permission or semantic detail.

**Tests:** Construct -> document -> object -> child orb -> params -> Back restores view/selection; edit remains; deleted target resolves safe parent; camera animation interrupted; saved view reload; no automatic authority expansion; CPU fallback; sheet/image/film depth examples. **Next:** ST-18/20.

### 18.20 ST-17 — Supervised-agent policy and G2

**D4 / SEC, RUST, SPEC, independent review. Depends:** G1, ST-06/09/11. **Read:** ST-G, existing governance/delegation/consent and intent/WAL facilities. **Write:** core `governance/app_supervision/` planned modules and execution-gate integration; no secrets/clinical policy work.

Implement typed task delegation, isolated proposal overlay, approval binding and lifecycle. Observe cannot mutate; Propose cannot publish or call external services; bounded mode checks scope/capability/effect/expiry/resource budget each boundary. Read actual principal identity; never accept model-generated grants. Approved digest binds exact target/params/revision and external summary. Pause/cancel/revoke have honest in-flight semantics. Idempotency/unknown remote outcome requires explicit retry decision. Add real accounting adapters; if missing, deny budget-dependent effects. Receipts bind actor and provenance; no false signed-audit claim.

**Tests:** every negative case in ST-G plus approval replay, proposal draft isolation, concurrent revision/approval and revocation races, transport reconnect, cancelled late result, malicious text as data, absent principal/accounting provider, immutable principal/scope, cross-app version and scope mismatch. Exercise CLI/MCP/worker/UI routes to prove no bypass. **G2:** independent review of exact policy and enforcement paths before agent-authorised mutations/provider actions; no low-cost self-approval.

### 18.21 ST-18 — Supervision panels and agent transports

**D2 / FE, QA after G2; D3 transport wiring. Depends:** ST-13/16/17. **Write:** shared UI `supervision/`, Poet macro task view/object proposal view, MCP/CLI transport adapters using ST-E only.

Expose typed inspect/discover/propose/approve/run/status/cancel via existing transport seams; authentication/authority comes from ST-17, not tool descriptions. Macro view shows tasks/scope/budgets/state; micro view shows target diff/preview, effect/cost and approval. Human and agent identities visually distinct. Agent may propose orb expansion, never click itself into more authority. Approve/revise/reject/pause/revoke accessible outside spatial rendering. Unknown cost/remote cancellation remains unknown.

**Tests/UAT:** same proposed document edit through agent API and human menu; reject leaves state; approve commits once; changed proposal/revision requires renewed approval; keyboard/list approval; revoked budget action fails; micro->macro retains receipts; GPU/daemon unavailable does not hide cancel/revoke. **Next:** all app proofs.

### 18.22 ST-19 — Generic app adapter and coverage machinery

**D2 / RUST, QA; D3 integration. Depends:** ST-01/02/07/08/09/11; G2 for delegated mutation proof. **Write:** one fork's `crates/poet-adapter`, engine catalogue resource generator and reference worker entry, then apply identical accepted scaffold to other forks without copying domain implementations.

Implement ST-E adapter methods and map actual upstream registries/handlers. Strict schemas come from accepted handler/test mapping, not guessed from JSON-ish descriptions. Inventory every UI-only action and gesture independently. Keep local engine model behind stable handles, domain snapshot via Q42 payload, semantic map, preview/display-list surface and error/warning adapter. File/clipboard/device/provider calls route through granted services. Jobs cannot freeze UI. Do not assume all apps share CommandSpec fields or registry location; PdfCraft automation table is separate and ArtCraft needs explicit command extraction.

**Tests:** command round-trip metadata; availability reflects selection; bad params cannot panic/mutate; unknown command; every mapped descriptor has handler/schema/test; all UI-only rows have destination; gesture one undo; worker/session isolation; snapshots/reopen stable IDs; no app-private authoritative DB. Cases run both worker backends. Contract test fixtures use synthetic document, not provider/model downloads. **Next:** ST-20/21.

### 18.23 Mandatory subunits for every app packet

Apply these suffixes to ST-20/21/22/23/25/26/27/29/31/32/33. A parent packet is a workstream, not a single 700-line session. Split each feature family into six-file units; log exact command IDs/coverage rows per unit.

- **a — Model/state adapter:** preserve complete editable baseline model, stable IDs, versioned snapshot/payload, semantic projection and originals; round-trip + malformed fixture tests.
- **b — Commands and gestures:** map all engine/UI-only IDs into strict descriptors, selection/availability, job/gesture/cancel; hostile-parameter sweep and undo assertions.
- **c — Content surfaces:** shared Dioxus container views, domain-native interactive surface, accessible tree/table/list, IME/focus/pressure as relevant; test real inputs, not merely rendering labels.
- **d — Tool/menu/orb/manifold:** contributions and seeds from shared registry; root/object/parameter contexts, menu focus and macro/micro return; no private command implementation.
- **e — Host, files and primitive adoption:** native/WASM services, mandatory domain cases below, shared Qualia primitive switch verified against oracle, explicit supported/loss/native-only matrix.
- **f — Conformance and coverage closure:** every baseline row resolved with evidence; full local workflow, agent approval workflow, crash/reopen/export/update cases, upstream focused corpus and once-per-app CI; independent reviewer checks missing rows. No percentage floor increase or claiming upstream completeness beyond source.

An initial thin vertical slice may run a/b/c/d/e for selected rows to validate architecture early; f must finish all baseline functionality. Record remaining rows and do not mark parent complete. Batch engine commands are not automatically atomic across documents; use ST-09 boundaries. Heavy computation stays in bounded jobs; preview state never replaces authoritative source.

For app subunit f, package-update assertions use the already accepted ST-06 development-package fixtures. Final signed release packaging is ST-37; it is not a prerequisite to closing an app implementation workstream. Otherwise ST-34/37 would depend circularly on app f. The ST-07/08 reference engine is a tiny synthetic session implementing the accepted contract for loader tests, not a prematurely migrated app; real engine wiring is ST-19.

### 18.24 ST-20 — GridCraft first full proof

**D2/D3; primitive semantics review. Depends:** ST-19, ST-13/16/18. **Source/write:** Grid row ST-J and fork adapter; shared sheet primitives only in separately claimed Qualia files.

First slice: import XLSX, select range, cell edit/formula recalc, chart, undo, save/reopen/export. Preserve Excel formula dialect/coercion/date systems and distinct optional Vibe projection. Virtualise grid and incremental calc. Complete all registered commands/UI actions under a-f. Native agent proposes a value/formula change; human reviews exact cell diff, approves, executes and sees same result in browser after reopen.

**Exact minimum assertions:** `=SUM(A1:A2)` updates when A1 changes; dependent recalculation only reflects committed revision; circular and spill collisions preserve explicit errors; row insert adjusts references; one fill drag one undo; date serial system retained; pivot/chart/validation survives XLSX; locale UI emits canonical syntax; unsupported unknown parts warn before overwrite. Large workbook quotas/refusal don't corrupt old document. **G3 part 1:** browser-local and native-hosted proof.

### 18.25 ST-21 — WordCraft full proof and G3

**D2/D3. Depends:** ST-19, ST-20 proof seam. **Source/write:** Word row ST-J.

First slice: DOCX import, edit text/style, paginate, comment/change tracking, contextual orb, save/reopen, DOCX/PDF export. Use rich document/layout engines, not contenteditable text as sole truth. CML is a parallel anchored semantic view; converting it cannot erase page/run/section/revision details.

**Minimum assertions:** split grapheme and bidi editing forbidden; IME commit one edit; list Enter/Tab matches baseline; table splits/repeated headers, page breaks/headers/footnotes retained; revision/comment anchors survive edit/reopen; DOCX round-trip and unknown/macro parts preserved without execution; PDF selectable text; glossary/citation/Zotero functions disposition recorded. Missing font fallback attributed. **G3:** independent proof of both apps, registry/menu/orb identity, approval, real durable state and cross-mode reopening; remaining apps follow stable interfaces.

### 18.26 ST-22 — DeckCraft

**D2/D3. Depends:** G3, ST-19. **Source/write:** Deck row ST-J.

Apply a-f to editable slide/master/layout/theme models, slide sorter/notes/presenter views, drawing/media/animation/presentation tool chains. Pointer/key engine routes replace shell-specific viewport logic. Distinguish editing state from fullscreen/presenter device state. Preserve opaque PPTX XML and legacy native extensions.

**Minimum assertions:** master change propagates overridden/nonoverridden placeholders correctly; transform/connector handles commit one operation; transition and keyframe timing survives reopen; presenter does not mutate slide source; embedded media synchronises; PPTX/PDF/native exports preserve declared capabilities; missing browser codec honest. Batch job cancellation retains prior deck.

### 18.27 ST-23 — PdfCraft

**D2/D3 integration; D4 signing/redaction/security review. Depends:** G3, ST-19/09. **Source/write:** PDF row ST-J.

Map both command and automation catalogues, preserve PDF object model/incremental writer and unknown objects. Domain containers include read/search/page organise, forms/annotation and security workflows. OCR/plugin/model capability unavailable is displayed, not mocked. Expose secure redaction as distinct from drawing a black rectangle. Signing uses provisioned host services; no private-key transport.

**Minimum assertions:** incremental edit leaves original bytes prefix intact; combine/split annotations/forms retained; save+encrypted recovery remains encrypted; signed document rewrite blocked/warned per source policy; redaction full rewrite removes text and obsolete objects from recoverable bytes; script sandbox denied external access; search respects reading order/bidi; malformed/decompression limits; no actual user certificate required in unit tests. Specialised security review precedes shipping sign/redact workflows.

### 18.28 ST-24 — Shared tiled imaging/colour/curve substrate

**D3/D4 / RUST, QA. Depends:** G1, ST-09, domain equivalence inventory. **Read:** core `hypermedia_authoring/image.rs`, render colour/gamut/texture APIs; Photo/Light/Vector raster/colour/compositor/text seams. **Write:** focused Qualia capability libraries with compatibility wrappers, not broad authoring-file growth.

Subunits: tile/pixel-format/mask references and bounded scratch; colour/ICC/alpha/blend parameter contracts; effect graph/cache invalidation; vector display-list/curve adapters; CPU oracle + GPU differential. Public buffers caller-owned; cold builders bounded. Preserve bit depth/channel count/sparse size/process versions. Adopt existing app algorithms with licence/attribution and tests; unify only equivalent semantics. Defer removed implementations until every caller switched and reviewed.

**Tests:** U8/U16/F32 source preservation; RGBA vs other channels; premultiplied/straight alpha edges; ICC round-trip/colour-oracle fixtures; tile bounds/overflow/eviction; mask operations; filter cancel and cache invalidation; CPU/GPU mismatch reports actual error. Use existing documented tolerances or reviewer-approved fixture tolerances; no widening to pass. **Next:** ST-25/26/27.

### 18.29 ST-25 — VectorCraft

**D2/D3. Depends:** ST-19/24. **Source/write:** Vector row ST-J.

Complete artboards/object hierarchy/appearance, path tools, type/brush/effect workflows and SVG/PDF/native/legacy format paths. Retain begin/preview/commit/cancel state machine; tool input emits shared commands. Accessible object tree exposes all paths even without GPU. Respect non-Adobe clean-room rules; synthetic fixtures only.

**Minimum assertions:** path boolean reference geometry; anchor edit single undo; cancelled pen returns original; appearance/symbol/blend/repeat remain editable; transformations precision; SVG/PDF export/reopen and legacy DrawCraft version; effect source/cache invalidation; text shaping/IME; all baseline menu and tool gestures covered.

### 18.30 ST-26 — PhotoCraft

**D2/D3. Depends:** ST-19/24/25 curve seam. **Source/write:** Photo row ST-J.

Complete tiled raster/layer/mask/type/vector/smart-object/filter workflows, high-bit-depth colour and PSD/PSB/native I/O. Reuse shared colour/tile core; engine still owns photo-specific document edits. Brush input includes pressure and bounded sample batching, separate preview/commit. Smart effects are live graphs, not flattened image exports.

**Minimum assertions:** PSD unknown blocks retained; PSB sparse large bounds no whole-image allocation; adjustment/mask/layer style output matches baseline CPU; U16/F32 channels and ICC intact; one stroke undo and cancel; smart filters remain editable after reopen; export equals approved preview within accepted oracle; plugin failure isolated; no missing-model ML claim.

### 18.31 ST-27 — LightCraft

**D2/D3. Depends:** ST-19/24/26 shared asset/colour seam. **Source/write:** Light row ST-J.

Complete catalogue/library metadata, albums/smart albums/stacks/virtual copies, RAW/non-destructive development/masking and batch export. Import original as immutable asset; path is a locator, digest is identity. Migrate journal/snapshot catalogues into ST-09 with mapping/validation/rollback, using synthetic fixture copies; no live user catalogue write.

**Minimum assertions:** old process version reproduces same edits; EXIF/XMP and ratings/keywords survive; virtual copies share original but not edit settings; torn catalogue write recovered; absent original relink checks digest; RAW preview/export consistent; mask/ICC and batch cancellation; missing optional model/camera profile honest; no face-recognition privacy claim from unprovisioned model.

### 18.32 ST-28 — Precision geometry and constraints

**D3 / RUST, QA. Depends:** G1, ST-09, ST-24 curve seams. **Read:** core computational_geometry/geometry_workspace, CAD geom/constraints/render; `q42/design_encode.rs` is not full CAD model. **Write:** focused Qualia geometry/constraints library adapters.

Preserve authoritative f64 world units and explicit unit conversion; f32 Tensor10D/render buffers are derived views. Bind entity IDs, layer/block topology, constraints and transforms to semantic/payload state. Reuse existing geometry before adding new solver; migrate app solver only if functional gap proven. Bound nested blocks/recursion/scratch and return contradiction diagnostics.

**Tests:** large and small coordinates retain requested units; transform inverse; snap tie determinism; associative dimensions follow edit; constraint conflict/underconstraint; degenerate paths; cyclic block refusal; caller-buffer full; deterministic parallel workspace partition/reduction. **Next:** ST-29.

### 18.33 ST-29 — CADCraft

**D2/D3; licence/platform review for DWG. Depends:** ST-19/28. **Source/write:** CAD row ST-J.

Complete drawing/paper-space/precision/annotation/layer/block/constraint and plotting actions; preserve interactive prompt factory, transparent-command and selection rules rather than reducing them to single buttons. DWG bridge remains native capability; browser can use authorised native-backed bridge or report unavailable. No invented mesh/solid editing absent from source.

**Minimum assertions:** chained interactive line/snap/keyword inputs, cancellation restores drawing; dimensions update after moving entity; constraints re-solve; DXF binary/ASCII round-trip and explicit unknown-entity loss; nested blocks; layout viewport scale/clip; f64 units; SVG/PNG/PDF plotting; DWG missing bridge does not pretend conversion. Keep MPL notices and separate bridge types.

### 18.34 ST-30 — Exact media time and realtime/offline DSP

**D3/D4 / RUST, QA. Depends:** G1, ST-09/24, host device adapter. **Read:** core audio/DSP/spectral and legacy authoring video; Film time/frame/project, Sound time/mix/model. **Write:** narrow Qualia time/audio/media/render capability modules with legacy conversion wrappers.

Use exact rational time as authoritative and checked integer conversion between rates; retain Film's exact tick basis at adapter boundary, preserve Sound sample/tempo map semantics. Replace reliance on legacy f64 video positioning with versioned precise records. Compile immutable DSP graphs/routing off realtime thread; callbacks use preallocated blocks/queues, no logging/allocating/blocking. Decoder/capture buffers stream/budget instead of adopting whole-source caches. Shared codec capability registry reports supported profiles/platforms.

**Tests:** exact common fractional rates and drop-frame boundaries; frame/sample alignment over long duration; tick overflow refused; timeline clip placement honoured; audio graph cycles/conflict; delay compensation; offline vs callback block equality; queue overflow/underrun reported; recording stops before buffer budget; preview/render cache invalidation; codec malformed/GOP seek fixtures. Benchmarks only after correctness and with accepted workloads. **Next:** ST-31/32.

### 18.35 ST-31 — SoundCraft

**D2/D3; D4 native plugin isolation. Depends:** ST-19/30/05. **Source/write:** Sound row ST-J.

Complete multitrack/mixer/routing/automation/MIDI/score/record/bounce. Native CLAP/VST3/AU scanning/instantiation/state has explicit allowlist and isolated lifecycle; browser does not claim native plugin availability. Preserve source model/metadata and original audio references. View close must not unexpectedly stop a different session's recording.

**Minimum assertions:** sample-exact split/fade; routing/tempo automation; one merged fader undo; delay compensation; offline stems and metering; missing plugin restores state as unavailable without loss; plugin crash cannot lose document/engine host; microphone consent/revocation; browser audio gesture startup; queued playback snapshot change; capture/disk-full recovery. Realtime allocation checks and worker/device tests are mandatory, not replaced by screenshots.

### 18.36 ST-32 — FilmCraft

**D2/D3. Depends:** ST-19/30/31 audio seam. **Source/write:** Film row ST-J.

Complete source/program/bins/timeline/scopes/captions, edit/trim/grade/effects/keyframes/nesting, media import/export and interchange. Services use range-backed reads, stepped/worker jobs, proxy/cache budgets and explicit CPU/GPU/WebCodecs selection. Do not adopt the current 384MiB browser cache as a Sentinel allowance. Keep pure-Rust codec fallback and platform-accelerated decoders behind the same contract.

**Minimum assertions:** exact 23.976 and 29.97 drop-frame clip/keyframe times; ripple/roll/slip and linked sound; sourceIn/sourceOut and nesting; waveform/scopes computed from actual output; GOP seek; preview/export CPU/GPU oracle; subtitle and interchange round-trip; export cancellation cleans only owned output; crash/reopen retains media links; absent WebCodecs/thread capability honest; main UI remains responsive under bounded job work.

### 18.37 ST-33 — ArtCraft

**D2/D3 local UI/services; D4 provider/external effects. Depends:** ST-19/24/28/30/17. **Source/write:** Art row ST-J plus frontend commands/state inventory. **Missing source:** production services repository; local work does not recreate it.

Subunits a-f apply, with extra g provider adapter. First extract typed commands/state from React/Tauri and build shared Dioxus composition/scene/camera/pose/mask/assets views. SQLite task rows migrate into shared job records with provider_job_id/provenance retained; test fixture migration only. Provider adapter uses authorised supported API and supplied credential handle; no cookie harvesting/browser emulation. Remove nonconsented analytics and implicit network startup. Return generated assets through shared governed asset identity/lineage.

**Minimum assertions:** scene/mask/pose snapshot; queued/running/failed/cancelled/deferred states; provider error/capability absent; timeout outcome unknown prevents automatic paid retry; idempotency exactly-once local dispatch; revocation/disclosure scope; no plaintext credentials/telemetry; response digest and source lineage; imported generation works as an editable asset in Photo/Film workflows. Paid/provider UAT is a principal-authorised separate packet, not a prerequisite for synthetic conformance or local tools.

### 18.38 ST-34 — Primitive consumption and full feature audit

**D3 / COORD, QA with domain reviewers. Depends:** completed app subunits, ST-24/28/30. **Read:** every coverage row, dependencies and primitive disposition. **Write:** focused adapter fixes and existing programme/ledger; one defect family per subunit.

Verify every promote_to_qualia item has designated shared implementation, licence and oracle evidence, and apps actually call it. Check whole-layer RGBA8/f64 timeline legacy models are not silently the authoritative destination. Remove no duplicate library until all callers/state versions accounted for. Verify command schemas cover UI-only actions/gestures, format matrix warns before overwrite, local/native support declared and no hidden app-private DB. Inspect dependency trees for unwanted toolkit/GPU/inference branches.

**Tests:** representative primitive invoked by at least two eligible consumers yields accepted same semantics; cross-repo contract versions match; every baseline row has exact evidence/disposition; no unsupported label conceals dropped feature; package-declared capabilities actually exist. Report unresolved rows by ID, never a guessed completeness percentage. **Next:** ST-35.

### 18.39 ST-35 — Cross-app workflows

**D2/D3 / FE, QA. Depends:** ST-34. **Write:** composition fixtures and end-to-end tests, smallest found adapters fixes.

Required workflows: Grid chart -> Word report -> Deck slide -> PDF review; RAW asset -> Light development -> Photo layers -> Vector artwork -> Deck/Film; CAD drawing -> shared scene/asset view -> documentation; Sound stems -> Film timeline; Art synthetic generation result -> Photo/Film; agent proposes an edit in one app with human approval while another linked view observes committed result. Links share asset/document identities and preserve original plus derivation provenance; copying pixels is an explicit export, not a live editable link.

**Assertions:** source revision propagation and cache invalidation; linked vs copied semantics distinct; cross-app permission denial; missing package/asset remains recoverable; checkpoint includes dependent payload references; reopen on browser/native; remove one app does not delete source; formats/colour/time/unit conversions carry warnings. Retain task-focused screenshots/receipts. **Next:** ST-36/37.

### 18.40 ST-36 — Domain collaboration

**D4 / RUST, ONT, QA. Depends:** ST-09/34, single-writer proofs. **Write:** domain-specific operation/merge modules and shared transport integration; one domain family per subunit.

Specify conflict semantics before editing: ordered text/spans; cells/formulas; object/layer order; clip ranges/track order; CAD constraints. Reuse existing CRDT facilities only where their operation semantics apply. A generic LWW object magnitude is not sufficient. Author/principal/effects stay authorised on replay/reconnect; immutable binary/plugin assets referenced by revision. Unsupported concurrent operation enters visible conflict/review state, not silent loss. Single-writer remains usable until domain concurrency verified.

**Tests:** same operation set delivered permuted/duplicated converges where contract promises; stale edit/deletion, insert/reorder, reference update, undo local vs remote, offline reconnect/revocation and asset availability; rich-text anchor validity; formula consistency; constraint conflict explanation. Independent reviewer accepts exact merge contracts. **Next:** release conformance.

### 18.41 ST-37 — Packaging, updates and compatibility

**D3/D4 / OPS, SEC, QA. Depends:** ST-06/34/35 and applicable collaboration. **Write:** package/build scripts, scoped CI and conformance fixtures in owning repos. No public publish/push/signing secret handling without principal approval.

Independent app build/package contains canonical manifest, digest-verified catalogue/engine/views/assets, licence/NOTICE and source baseline. Lock supported dependency/profile matrix; no newest floating dependency upgrade/security bypass. Browser local install/cache and native managed install share manifest/schema/permissions. Upgrade dry-run backs up state, checks process version, validates migrations and can roll back. Preserve documents when app missing/removed. Package size/resource evidence is measured; no arbitrary thresholds substituted for existing policy.

**Tests:** engine/API/schema mismatch refused; digest/signature failures; old version reopen; upgrade/downgrade and failed migration rollback; interrupted package write; multiple apps independent; offline asset/font path; exact wasm-bindgen glue pin; native plugin architecture mismatch; licence obligations retained. Run per-app full established CI once here if not run at f closure; full-suite final gate once in ST-39, not every session.

### 18.42 ST-38 — Retire legacy Desktop UI safely

**D3 / COORD, RUST, QA; D5 removal/deletion. Depends:** OS-shell native/browser hosting, ST-03/04/05/10/37. **Write:** launch/default route/config adapters and scoped deprecation notices; no deletion of legacy source/user state in ordinary packet.

Service coverage checklist: graph/Vibe/session binding, managed app lifecycle, files/clipboard, notifications, capture, native surface, telemetry, companion transport, updates, recovery, identities/permissions. Every necessary service has implementation tests and an OS-shell caller. Obsolete domain commands are redirected to their app, not copied into shell. Change default launch after parity UAT; keep reversible legacy adapter/reference build until principal authorises removal. OS shell must not import the whole webizen-desktop executable/library or require Tauri to run. Dioxus native webview backend is explicitly documented, not falsely claimed absent.

**Tests/UAT:** start new shell with legacy executable absent; mount native Poet; same document opens browser; service offline/failed; capture and GPU shutdown; cross-mode checkpoint; compatibility entry reports deprecation/redirect; no user-data moves/deletion. **Next:** G4.

### 18.43 ST-39 — Final G4 evidence review

**D4 independent QA/SEC, D5 release decisions. Depends:** all prior parent packets; approved exceptions explicit. **Write:** ledger, review defects, release checklist only; fixes get bounded follow-up packet IDs.

Review full diffs and evidence, not summaries. All eleven apps have baseline coverage, native/WASM declarations, shared primitive consumers, format/security fidelity, durable state and supervised-agent workflows. Orb/menu/palette/toolchest execute same commands; attention restores view; CPU/accessibility fallback usable. OS-shell real hosting and extracted services replace old UI. Run one agreed broader suite per changed workspace/profile after narrow fixes pass; hardware/corpus/externally provisioned omissions remain named, not counted green. Existing security controls and NQuin/42MB/zero-heap contracts intact. No provider costs/public release without authorisation.

**Completion:** reviewer signs evidence-based G4 record; principal accepts any legitimate unsupported/native-only release exceptions and external publication. Planning text, fake statuses, skipped tests and raw test counts do not close the gate.

### 18.44 Continuation prompt (copy verbatim and replace the packet ID)

```text
Execute only ST-<ID>/<subunit> from section 18 of
C:\github\qualiaDB\docs\POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md.
Read the ST programme chapter in the linked implementation plan, the assigned
packet, applicable repository rules, and the existing session ledger.
Use the packet's actual repository roots, not legacy monorepo Poet copies.
Inspect HEAD/branch/status in every repo you will touch; preserve user changes.
Check dependencies and evidence first. Do not redo completed valid work.
Keep one bounded subunit: at most six implementation files plus tests/ledger,
new implementation files under 500 lines, no more than 700 net lines.
Implement the named interfaces and tests, not a guessed alternative.
Use targeted verification; a filter with zero tests is not evidence.
Do not widen frozen Host, invent a manifest or capability, change security
controls, publish, push, spend on providers, delete user data, or access secrets.
If authority/licence/ABI/environment blocks the subunit, record the exact
blocker and next action; do not silently drop it or claim completion.
Append an honest ledger row with all repo commits, changed paths, exact commands,
results, UAT/artifacts, remaining coverage rows and recommended next subunit.
Stop after the assigned subunit. No implicit commit or push.
```

### 18.45 Planning delivery record

2026-10-11: ST specification and packet queue written into existing programme documents; Poet entry point linked. All ST implementation packets begin NotStarted until execution evidence is appended. No product builds, migrations, app forks, packages, provider calls or UI implementation are asserted by this planning delivery. Start with ST-00; do not execute the September HLT-R1 starting instruction for this programme.

## 19. VB programme — VibeScript JavaScript-replacement extensions

**Authority:** implementation specification ST-N and current normative `docs/manuals/standards/vibescript-core.md`. These ten packets supplement ST-00 through ST-39; use the same bounded session, independent review, evidence and authority rules. Implementation has not started. Vibe is a growing application language; stable parse/check/diagnose/invoke facade does not freeze new language/library/capability support. Do not use old freeze wording to reject a necessary reviewed binding. Vibe is not an ECMAScript clone and gains no DOM, eval, ambient credentials or JIT.

### 19.1 VB-00 — Required scripting behaviour and gap baseline

**D3 / SPEC, QA. Depends:** ST-00; execute before G1 closes. **Read:** core language sections 0/4-6/8/11/14-17; live `vibe/src/{lib,types,value,module_system,presentation,budget}.rs`, `eval/{mod,expr,stmt}.rs`, catalogue and WASM serialization; Storytold ArtCraft TS/state/event/service code, app web/worklet bridges, Poet browser handlers. **Write:** ST-N/ledger and app coverage resources established in ST-01, no implementation.

For each required behaviour create the row described in ST-N3: source/trigger/types/effects, current Vibe mechanism, actual native/WASM test, missing syntax/library/host/scheduler/tooling seam and required assertion. Inspect real await/yield scheduling, module graph vs execution imports, typed Value interop, lambda restrictions, reactivity and machine-schema honesty. Do not replace implemented features based on stale spec or count test declarations as execution proof. Identify all handwritten JS/TS versus generated glue and platform-only worklets. Baseline focused commands `cargo test -p vibe --test conformance`, `--test vibe_features`, `--test vibe_catalog_surface` from QualiaDB root, and target checks only as needed; record unrelated failures.

**Acceptance:** each app-script requirement classified reuse/binding/library/dialect/platform bridge; no fake full-JS/no-JS claim; exact small extension list and interface proposals fed into G1. AST Await identity and JS Debug serialization fallback have regression scenarios. **Next:** VB-01/02/03 and approved feature subunits VB-04.

### 19.2 VB-01 — App/job/UI capability bindings

**D3/D4 / RUST, SPEC, SEC. Depends:** G1, ST-11; ST-17/G2 before delegated mutation enabled. **Read:** Vibe catalogue/`capability_schema.rs`/effects/eval-call; core `poet_host/invoke/{ids,mod,coverage}.rs`; ST-E typed requests and job/authority services. **Write:** focused family adapter and paired catalogue/schema/discovery rows, never the wide Host trait.

Proposed v1 family surface to review in G1, NOT existing exports: `App.commands`, `App.availability`, `App.inspect`, `App.execute`, `Job.status`, `Job.cancel`, `UI.describe`, `UI.propose`. If an equivalent ID already exists, retain that canonical name and record the mapping; do not implement duplicate aliases with different authority. Describe/read take a typed session/target and return bounded catalogues/snapshots; execute takes the full ST-E CommandRequest and returns receipt or job handle; status/cancel take job/session and actor authority; UI.propose returns a validated presentation/proposal, not an automatically applied grant.

App.execute is effectful and forbidden in Pure/Hot cells even if params name a read command; snapshot-safe read binding has independently enforced semantics. Dynamic command IDs are resolved by shared registry and effects/target/permission revalidated at runtime. Import/using declaration is not authorisation. Value conversion preserves exact integers/time/unit/opaque handles and explicit errors.

**Tests:** unknown/unbound/dynamic ID; schema-handler agreement; malformed/missing/extra params; unavailable host; Pure/Hot mutation blocked; wrong scope/principal/revision; UI/agent/Vibe execute same handler and receipt; proposed UI never mutates; cancellation honest. Native/WASM discovery labels actually reachable implementations. Run narrow Vibe catalogue/effects tests and core family tests plus one affected target check. **Next:** app ST-19 and VB-02/05.

### 19.3 VB-02 — Real bounded async/events/streams

**D4 / RUST, SPEC, QA. Depends:** VB-00/01, ST-09 accepted revision/job interfaces. **Read:** `vibe/src/eval/{mod,expr,stmt,program,call}.rs`, AST/types/effects/budget, host job/event transport. **Write:** directory-backed `vibe/src/async_runtime/` planned module, focused eval integration, core/host task adapters. Reviewer supplies exact continuation/codec/version before implementation.

Replace identity-await behaviour for genuinely pending jobs with ST-N5 state machine: explicit Ready/Pending/Cancelled and budget retained across resumes. Define typed opaque Future/Stream/job/subscription representation; cancelled/closed/revoked tasks release resources. Event order/queue/backpressure deterministic and finite, no JS event-loop dependency or resetting gas. Native waits outside UI thread; WASM yields the execution turn. No open graph transaction, mutable lock or unapproved external replay across await. Checked sync await may only accept an already-ready value under the accepted type rule; unknown/pending value cannot silently be treated complete.

**Tests:** pending job yields then resumes with actual result; exception/result error propagation; two tasks ordered correctly; queue overflow; subscription dispose/reload; cancellation before/during/after completion; revoke and stale revision after resume; late reply dropped; timeout unknown external outcome; maximum steps/workspace survive every yield; Pure await rejected; transaction-await rejected; UI tick remains responsive. AST/native/WASM receipts and recorded-clock replay match. Bytecode unsupported async refuses/falls back explicitly, never executes a no-op. **Next:** VB-06/08.

### 19.4 VB-03 — Executable versioned modules and standard-library parity

**D3 / RUST, QA. Depends:** VB-00, G1. **Read:** `module_system.rs`, `eval/mod.rs::populate_import_aliases`, parser imports, canonical module/package resource rules. **Write:** module resolve/load/check/eval integration and bounded standard-library functions only for inventoried gaps.

Connect module graph metadata to actual import execution: package-provided digest/version-pinned resources, export/type checks, cycle detection, deterministic lookup, offline imports. No npm solver/implicit network fetch/ambient file import. Preserve existing `vibe:0.1/*` imports; do not expose every catalogue family as a keyword. Required list/record/string/bytes/numeric/error functions have one pure bounded implementation and declared effects; checked overflow/division/Unicode semantics retained. Missing module/version/capability gives useful stable diagnostic, not fallback to JS.

**Tests:** module function executes across imports; alias collisions; cyclic/transitive missing import; tampered digest/wrong version; private export; native/WASM same value and effect; collection overflow; Unicode slicing; exact u64; Err/None propagation. Package imports cannot widen capability scope. **Next:** VB-04/07/08.

### 19.5 VB-04 — Required dialect/type/effect additions

**D4 specification, D2/D3 implementation after accepted design. Depends:** VB-00 exact gaps, G1. **Write scope:** one feature through the ST-N4 pipeline; language-only change, no app rewrite mixed in.

For each feature the SPEC owner writes exact syntax/EBNF, AST/type/effect rules, runtime state transitions, positive/negative examples, diagnostic codes, budget behaviour, version compatibility and interpreter/bytecode disposition. Do not ask an inexpensive implementer to choose language semantics. Prefer existing explicit-state functions/noncapturing lambdas and Result/match over capturing JS callback semantics. If a required closure/collection/type/event feature genuinely needs growth, document ownership/lifetime/replay and bounded memory before coding. Reconcile live interpolation/other implemented features with outdated core notes instead of adding a second implementation.

Implementation order: regression fixture -> grammar/schema -> lex/AST/parse -> check/effects -> eval -> CBOR/bytecode/formatter/projection -> CLI/LSP/WASM -> examples. Scope splits may span sessions but no half-supported feature is advertised. Breaking old source/AST/opcode semantics needs accepted new version and old-version handling; no silent bump from branch name.

**Tests:** accepted/rejected syntax; type/effect failures; source spans; round-trip parse/pretty/CBOR; malformed/truncated/unknown-version AST; old dialect preserved; numerical/identity exactness; budget/recursion refusal; AST/bytecode same result or explicit unsupported. Non-JIT and raw-Quin restrictions remain. **Next:** VB-07.

### 19.6 VB-05 — Vibe-authored menus, orbs and reactive UI

**D2/D3 / FE, RUST, UX. Depends:** VB-01/03, ST-10/13/14; VB-02 for async progress/subscriptions. **Read:** normative UI presentation section, `vibe/src/presentation.rs` and reactive cells; ST-E/F descriptors; shared Dioxus models. **Write:** shared UI Vibe projection/reconciler and Poet module editor integration, not a new JS/DOM renderer.

Accept bounded keyed declarative records/presentations and render the canonical components. Allowlisted types/properties/tokens, escaped labels, shared target/context/command IDs, accessible projection; no arbitrary HTML/CSS or callback strings. Hot reload parse/check/validate before atomic replacement, retain last good module on failure, dispose old subscriptions/resources, retain compatible editing state. Menus/orb child groups originate from same descriptors; no label-based dispatch or self-approved action.

**Tests/UAT:** edit `.vibe` label/layout/action and see native/browser update without Rust host rebuild; invalid source retains old view/diagnostic; duplicate key/depth/size/unknown node refused; event carries correct revision; required async job progress/cancel; reactive dependency update/cycle; keyboard/tree equivalent; no repeated external request on reload; proposal/approval invalidated where required. Demonstrate one panel, contextual menu contribution, child orb and agent proposal view. **Next:** all app script migrations.

### 19.7 VB-06 — Typed native/WASM host I/O and exact interchange

**D3/D4 / RUST, SEC. Depends:** VB-01/02, ST-05/07/08/09. **Read:** `vibe-wasm/src/lib.rs`, Value and typed CBOR serializers, host providers and caller-buffer APIs. **Write:** typed serialization and narrow file/asset/graphics/audio/device/network adapters; no business logic in generated glue.

Replace Debug-string fallback as the production interop path with explicit tagged/versioned exact types/opaque refs for all supported values. Dense media stays in authorised payload handles; vectors/arrays are bounded. Browser/API adapters are explicit and minimal; granted requests return receipts. Record necessary generated loaders/worklet shims, pin versions and test; migrate handwritten app business logic out of shims. Browser-local denied network does not quietly connect native host.

**Tests:** i64 min/max, u64 above 2^53 and max, hashes/ticks, exact duration/time scale, IRI vs string, Result/Option, nested records/lists, bytes and asset/tensor/geometry refs; malformed/tag/version/oversize rejection; native->WASM->native round-trip; no debug repr as state. File/path/capture/network denial, expiry/revoke, device loss and job cancel; valid scripted render/audio job through real adapters or capability-unavailable label. **Next:** VB-08/09.

### 19.8 VB-07 — Diagnostics, editor, CLI and documentation

**D2 / FE, QA after accepted runtime changes. Depends:** relevant VB-01/03/04 features; VB-02 scheduler diagnostics when implemented. **Read/write:** focused `vibe-lsp/src/{catalog_intel,server}.rs`, canonical Poet native CLI formatter/linter/REPL, browser console/editor and normative/examples resources.

Generate completions/hover/signatures from real accepted machine schemas; unknown ID gets a useful suggestion but never invented bind. Diagnostics preserve code/UTF-8 byte span, effect/capability reason, source version and actionable correction. Editing hot-reloaded modules does not rebuild host. CLI and UI accept same dialect/modules and show native/WASM availability. Debugging exposes bounded read-only task/stack/variables/receipt refs with private values redacted; inspection does not evaluate expressions with effects.

**Tests:** parse/check/diagnose parity across native CLI/browser/LSP; Unicode source-span mapping; new capability signature/effect; missing module/version; async pending/error/cancel; malicious source suggestion escaped; example modules all parse/check and execute against authorised synthetic hosts; documentation does not mark catalogue frozen or all JS semantics implemented. **Next:** VB-09.

### 19.9 VB-08 — Per-app JavaScript/TypeScript migration

**D2/D3 after accepted language/host seams. Depends:** app ST a-e, VB-01/03/05/06 and required VB-02/04. **Write:** one app's `.vibe` modules, adapter state/actions and focused tests; six-file bounded family per subunit.

Consume VB-00 behaviour inventory, most extensively ArtCraft React/Zustand/Tauri orchestration and browser callbacks. Port intended state transitions to explicit typed app state, pure transformations, hooks/events, awaited jobs and shared command binding. Declarative view scripts feed shared Dioxus, not React emulation. Rust handlers remain compiled primitives where appropriate; authorable workflows/presentation/event logic are Vibe. Keep old handler only as labelled reference until its assertions pass, then route active UI solely to Vibe/shared dispatcher. No general transpiler or a Vibe wrapper calling the old JS business function as completion.

**Tests per row:** same synthetic input/state/trigger -> expected domain result/effects; errors/denials/cancel/timeouts; reload/reopen; exact values; reactive dependency and disposal; no repeated remote side effect; native/browser/script/menu/agent same command receipt; no silent JS fallback. Every retained handwritten JS row has minimal platform-only reason, owner and tested interface. App ST-f cannot close with required script rows missing. **Next:** VB-09 and ST-34/35.

### 19.10 VB-09 — Application-language conformance and G4

**D4 independent QA/SPEC/SEC. Depends:** all required VB features and app VB-08 rows, ST-35/37. **Write:** conformance evidence and bounded correction packets; no new language design in final review.

Run native and wasm real interpreter workflows: module/import, collection/error semantics, reactive state, event/UI callbacks, job/stream/await/cancel, command/approval dispatch, data/asset I/O, graphics/audio, script hot reload and exact-value interchange. Pure/Hot/Cold/Async/External policy is enforced; declared availability matches execution. AST and optional bytecode agree or explicitly refuse unsupported features; no no-op async, unchecked memory/gas, Debug-value state or second scripting language. Run established Vibe conformance once at gate plus relevant WASM/browser tests, without rewriting expected outcomes/floors to mask failures.

**Release claim:** Vibe fully supports the inventoried Qualia/Poet/Webizen app-language requirements. Report residual platform glue separately and do not claim ECMAScript compatibility or no-JavaScript deployment. G4 incorporates VB evidence and remains open while any required language/runtime/app-script row is missing. Exact gap IDs, not a synthetic percentage, guide next work.

### 19.11 VB continuation prompt

```text
Execute only VB-<ID>/<subunit> from section 19 of the current execution playbook.
Read implementation specification ST-N and the named normative Vibe sections.
The language and capability catalogue may grow; preserve the stable four-op
facade and version old source/AST/bytecode/effect meanings explicitly.
Use the assigned exact feature contract/tests. Do not invent JS/DOM/eval APIs,
embed another script language, bypass command supervision, reset budgets on
await, or hide business logic in loader/worklet glue.
Check actual native/WASM execution, not parser success or catalogue presence.
Follow section18 session/evidence/authority rules and append the shared ledger.
Do not implement the next feature, commit, push or invoke external providers.
```

2026-10-11 planning addition: VB-00 through VB-09 are NotStarted. Language/runtime gaps are specified for future regression tests; no Vibe feature was changed or runtime-conformance result claimed during this planning task.
