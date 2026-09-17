# POET non-QApps continuation handover and executable to-do list

**Date:** 2026-08-29  
**Status:** Active implementation programme; do not claim full completion  
**Primary tracker:** `docs/POET_PRODUCT_INTEGRITY_REMEDIATION_2026-08-29.md`  
**Parity/history tracker:** `docs/POET_UI_PARITY_IMPLEMENTATION_2026-08-27.md`  
**QApps:** Separate active migration into native POET constructs/manifolds; see
section K of the parity tracker. Do not recreate or extend a parallel QApp runtime.

This file is deliberately explicit so a less capable implementation agent can
continue without asking the principal to reconstruct the product requirements.
Keep this file updated after each coherent implementation batch.

## 1. Objective

Fully implement the useful POET application, excluding direct QApp migration
work. “Implemented” means a person can complete the advertised job through a
domain-appropriate UI backed by real state and real operations. A generic record
form, a disabled button, a toast, a route, or a passing unit test is not by itself
a completed feature.

The QApps programme is not cancelled or abandoned. It is running separately and
must migrate legacy QApps into POET-native constructs, manifolds, typed
containers, Semantic Library entries, and HCF/HMC packages while preserving
their actual user jobs.

## 2. Mandatory working rules

1. Read root `AGENTS.md` before changing code. Its memory, ABI, modality, file
   size, temporary-file, and non-adversarial rules are mandatory.
2. The worktree contains extensive principal-owned/concurrent changes. Never
   reset, checkout, delete, or mass-format unrelated files.
3. Use `apply_patch` for source/document edits. Preserve overlapping work.
4. New Rust behavior belongs in focused files below 500 lines. Do not add more
   behavior to a file already above 1,000 lines without recording and beginning
   a decomposition in this programme.
5. Do not put heap allocation into QualiaDB Tier-1 hot paths. Browser/service
   code may allocate within its bounded cold-path contracts.
6. Deliver coherent user jobs in batches. Run one proportional native/browser
   compile after a batch; do not spend most of the session repeatedly launching
   tests or narrating them.
7. Never label an unprobed transport “connected”, an unsigned record “signed”, a
   persisted amount “paid”, a model assertion “verified”, or a local write
   “delivered to another host”.
8. Update this handover and the primary tracker after material implementation.
9. QApps work belongs in the parallel migration programme. Non-QApps work must
   not wait for it, and QApps must not be represented as abandoned.

## 3. Current verified build baseline

As of the timestamp above, these commands pass:

```powershell
cargo check -p qualia-core-db --lib
cargo check -p poet --target wasm32-unknown-unknown
```

System-Chrome task-level UAT is still open. The Chrome controller can inspect
the retained page but click dispatch previously timed out before reaching POET.
Do not convert that limitation into a claim that browser UAT passed.

## 4. Important implemented foundations — do not rebuild them

### 4.1 Shared POET application foundation

- Standalone POET Tool Chest exposes first-class Social, Local AI, Connectors,
  Semantic Library, Budget, and the broader registered POET surfaces.
- Shared daemon discovery, capability negotiation, invocation, record CRUD,
  Semantic Library ingest/query, Pulse SSE, graph events, and render-preview
  adapters live in `crates/poet/src/browser/native_daemon.rs` and related modules.
- COP records are bounded, family-allowlisted, persisted by the daemon, and
  validated in `crates/qualia-core-db/src/services/poet_record_api.rs`.
- Semantic Library browser UI is integrated with the real native backend.
- Capability negotiation includes machine argument/return schemas, semantic
  family, effect class, honesty, transport, and availability.

### 4.2 Local AI and agent operations already implemented

- Real GGUF/P64 inference through the existing guarded local runtime.
- Optional Semantic Library grounding and up to eight prior conversation turns.
- Named conversations and bounded persisted turn restoration.
- `/llm/jobs/start`, `/llm/jobs/events`, and `/llm/jobs/cancel` implement bounded
  jobs, sequenced token SSE, retained terminal events, and cooperative decode
  cancellation.
- Native and fallback decode loops observe `DecodeControl`; cancelled output is
  displayed as partial but is not persisted as a completed turn.
- Explicit per-run token budget of 1–256 is enforced inside the decode loop.
- `/llm/models/activate` and `/llm/models/evict` operate on the real resident
  model slot. Active jobs block model replacement/eviction.
- Failed activation no longer clears the previously resident model before the
  replacement has loaded successfully.
- Completed/cancelled/failed runs persist as `project_agent_run` receipts,
  separate from conversation turns.
- Completed model assertions persist with `review_status=pending`; reviewer-DID
  approve/reject records review without triggering tools or publication.
- Optional `project_agent` profiles with `kind=profile` enforce owner DID,
  permitted model, token ceiling, `local-inference`, and exact Semantic Library
  scope (`none`, `all`, or `project:<tag>`) at the daemon boundary.
- Key files:
  - `crates/qualia-core-db/src/services/poet_llm_api.rs`
  - `crates/qualia-core-db/src/services/poet_llm_jobs.rs`
  - `crates/qualia-core-db/src/inference/inference_agent/control.rs`
  - `crates/qualia-core-db/src/inference/inference_agent/decode.rs`
  - `crates/poet/src/browser/project_views/agent_console_workspace.rs`
  - `crates/poet/src/browser/project_views/agent_session_browser.rs`
  - `crates/poet/src/browser/project_views/agent_review.rs`
  - `crates/poet/src/browser/project_views/agent_run_history.rs`

### 4.3 Social already implemented

- DID-attributed persisted messages with thread/channel filtering.
- Live Pulse publication notice after local message persistence.
- Semantic channels with creator DID, topic IRI, visibility, and membership
  policy (`open`, `request`, `invite`).
- Open membership, creator-decided join requests, and creator-issued invitations
  for invitation-only channels.
- Only the receiving DID may accept/deny/block a request.
- Request principals/type/scope are immutable after creation; terminal decisions
  cannot be rewritten.
- Acceptance creates the correct active `manifold_participant` record.
- Restricted-channel posting is membership-gated.
- Scoped, expiring voluntary presence and recent Pulse activity are visible.
- Key files:
  - `crates/poet/src/browser/social_workspace.rs`
  - `crates/poet/src/browser/social_lifecycle.rs`
  - `crates/poet/src/browser/social_inbox.rs`
  - `crates/poet/src/browser/social_presence.rs`

### 4.4 Connectors already implemented

- Discover and run negotiated native capabilities.
- Persist semantic connector descriptions with interface/input/output IRIs,
  transport, auth mode, sensitivity, capability/endpoint, status, effect class,
  input schema, output schema, and saved probe arguments.
- Reconcile saved connectors with the live capability catalogue.
- Publish connector descriptions as JSON-LD into the Semantic Library.
- Persist bounded `project_connector_run` receipts.
- Failed runs can be retried only for declared `Pure` effects and only through
  attempt three. Cold/unknown operations require a fresh explicit run.
- External transports cannot be marked enabled until a real host adapter probes
  them.
- Key files:
  - `crates/poet/src/browser/project_views/connector_workspace.rs`
  - `crates/poet/src/browser/project_views/connector_health.rs`
  - `crates/poet/src/browser/project_views/connector_runs.rs`

### 4.5 Project economics already implemented in useful part

- Separate plan, actual, funding, royalty, and tax ledgers.
- Fixed six-decimal, unit-separated derived totals and variance.
- Lifecycle-aware exclusions prevent drafts/observations/cancelled/settled rows
  from being misrepresented.
- Current-daemon audit JSON export with explicit non-payment/non-settlement
  warning.
- Backend requires actor, unit/currency, sensitivity, effective date, and valid
  lifecycle semantics.
- Key files:
  - `crates/poet/src/browser/project_views/budget_workspace.rs`
  - `crates/poet/src/browser/project_views/budget_model.rs`

## 5. Ordered non-QApps to-do list

Status markers: `[ ]` not started, `[~]` partially implemented, `[x]` verified
implemented slice. Do not turn a parent item `[x]` until every acceptance item
under it is complete.

### P0 — communications people can actually use

- [x] Add message replies.
  - Add optional `reply_to` message ID to the Social composer.
  - Backend must verify the referenced `social_message` exists and is in the
    same thread/channel.
  - Render reply context without unsafe HTML.
  - Implemented in `social_workspace.rs` plus atomic backend validation; reload
    preserves the relation and invalid/cross-thread IDs fail.
- [x] Add DID mentions and notification records.
  - Composer accepts a bounded list of DID mentions (maximum 16).
  - After message persistence, create one `social_notification` per mentioned
    DID, referencing the persisted message ID and thread.
  - Implemented as bounded, deduplicated mention DIDs and one local receipt per
    recipient after the real message ID is returned. This is not remote delivery.
- [x] Add notification unread/read state.
  - Build a focused `social_notifications.rs` component.
  - Query only notifications addressed to the entered actor DID.
  - Only the recipient may mark a notification read.
  - Implemented in focused `social_notifications.rs`; recipient/source/thread
    are immutable and read cannot transition back to unread.
- [~] Add channel administration and moderation.
  - Creator can appoint/remove moderators through participant-role transitions.
  - Moderator/creator can hide a message with an attributed moderation receipt;
    do not physically delete evidence.
  - Implemented: creator-only participant role transitions preserve immutable
    participant/manifold identity and record actor/time.
  - Implemented: creator/active-moderator hide receipts preserve source
    messages; rendering replaces bodies/reply excerpts with moderation notices.
  - Remaining: blocked relationships must prevent new direct requests/messages
    where the current transport can enforce it.
- [ ] Add attachments.
  - Use the existing Semantic Library ingest path and store an attachment URI,
    media type, sensitivity, and provenance on the message.
  - Do not embed unbounded binary data in COP records.
- [ ] Add encryption/signature and real receipt semantics.
  - Reuse existing DID/crypto facilities; do not invent placeholder signatures.
  - Separate local persistence, queued transport, delivered, read, and verified
    states. Until a multi-host adapter exists, label delivery unavailable.
- [ ] Add real multi-host transport/session integration and task-level Chrome UAT.

### P0 — agent tool authority and usable operations

- [ ] Add scoped connector/tool grants to agent profiles.
  - Do not let a comma-separated UI string alone authorize execution.
  - Persist explicit grant records tying principal, agent DID, capability ID,
    effect class, sensitivity ceiling, expiry, and allowed argument schema.
  - Validate grants at the daemon invocation boundary.
  - Pure read tools may be directly permitted; Cold/external/economic effects
    require an explicit per-run human approval state.
- [ ] Add an agent tool-approval queue.
  - Show exact capability, arguments, effect class, sensitivity, principal,
    agent, and expected result schema.
  - Approve once, deny, or cancel. Approval must be consumed and auditable.
  - Never treat assertion review as tool approval; these are different states.
- [ ] Add aggregate resource budgets.
  - Per-profile rolling limits: tokens, wall time, concurrent jobs, and optional
    connector invocation count.
  - Enforce in the daemon, not only the browser.
  - Show remaining/consumed budget using persisted run receipts.
- [ ] Add bounded model installation/import.
  - Browser cannot upload arbitrary multi-GB content into a JSON endpoint.
  - Prefer selecting/registering a host-visible GGUF/P64 path or a bounded
    streaming import into a configured model directory.
  - Validate extension, file magic, byte budget, duplicate identity, and
    destination containment. Never overwrite an existing model silently.
- [ ] Add durable job inspection details.
  - Run list should expose job ID, model, profile, token budget, generated
    tokens, duration, terminal reason, context/project scope, and provenance.
  - Add filters by agent/status/date without loading unbounded history.
- [ ] Add browser UAT for select → activate → run → stream → cancel → review.

### P0 — connectors as semantic, dependable integrations

- [ ] Generate forms from negotiated input machine schemas.
  - Support bounded object/string/number/integer/boolean/list/null shapes.
  - Show required fields and local validation errors before invocation.
  - Retain a raw JSON expert mode.
- [ ] Validate connector output against return schema and semantic output class.
  - Persist validation outcome separately from transport success.
- [ ] Implement semantic mappings.
  - Map connector-native input/output to declared Semantic Library classes.
  - Store mapping provenance and reject lossy/invalid mappings unless the user
    explicitly accepts the limitation.
- [ ] Implement authentication ceremonies.
  - Capability/DID-signature: use existing local authority systems.
  - OAuth: host adapter owns browser redirect/token storage; never store tokens
    in COP descriptor/run records.
  - MCP/HTTP/WebSocket/Pulse/file: each needs a real adapter with separate
    configured, authenticated, probed, and enabled states.
- [ ] Add connector health scheduling/backoff and revocation.
  - Retain bounded receipts; do not retry non-Pure effects automatically.
- [ ] Make published connector JSON-LD graph-queryable by agents and workflows.
  - Verify query by interface, input/output class, effect, sensitivity,
    authority, transport, and current health.
- [ ] Perform external-adapter and browser UAT.

### P1 — project delivery workflows

- [ ] Restore interactive Kanban rather than grouped CRUD rows.
  - Drag/explicit transition with actor, previous/new status, timestamp, and
    optimistic-conflict handling.
- [ ] Implement task dependencies and cycle detection.
- [ ] Connect milestones, roadmap, Gantt, and calendar to the same task graph.
- [ ] Implement risk → mitigation → owner → review lifecycle.
- [ ] Implement deliverable acceptance, evidence, backlinks, and version history.
- [ ] Link compensation and obligations into Budget projections.
- [ ] Add approval authority transitions for economic commitments.
- [ ] Add append-only audit/export and Chrome task UAT.

### P1 — Agreement, Rights, Governance

- [ ] Restore agreement drafting/party/negotiation/ratification lifecycle.
- [ ] Connect deontic norms and N3 compilation to visible obligations/permits/
  prohibitions/defeaters with expiry and evidence.
- [ ] Implement explicit consent, delegation, revocation, signature thresholds,
  and `SuspendedTransactionQueue` consensus flows where applicable.
- [ ] Restore rights/licensing terms, contribution attribution, compensation,
  breach, remedy, dispute, correction, and appeal workflows.
- [ ] Preserve evidence and authority receipts; do not infer legal effect from a
  saved record.
- [ ] Add independent-review export and Chrome UAT.

### P1 — Dataset, Ontology, Knowledge, Semantic Library

- [ ] Restore visual lineage graph with selectable nodes/edges and provenance.
- [ ] Restore mapping editor with real source/target terms and validation.
- [ ] Restore SHACL/ShEx result navigation to the offending graph datum.
- [ ] Restore semantic relation and vocabulary editing around the real graph.
- [ ] Add Library backlinks, versions, duplicate/identity handling, sensitivity,
  project/purpose scopes, and auditable publish/update flows.
- [ ] Ensure natural persons are never modelled as `owl:Thing`/`owl:Class`.
- [ ] Add task-level browser UAT.

### P1 — Health/person-controlled records

- [ ] Replace generic mini-EHR forms with person-controlled timeline workflows.
- [ ] Implement measurement units, reference/provenance, corrections, and
  uncertainty without fabricated values.
- [ ] Implement disclosure request → consent → scoped share → revoke → audit.
- [ ] Enforce classified/secret sensitivity and named clinician/guardian DID.
- [ ] Wire real document extraction/OCR/PDF support when backend is available;
  never pretend raw binary was parsed.
- [ ] Restore safeguards, attestations, welfare support, and hypotheses as
  coherent person-controlled workflows.
- [ ] Add clinical-safety and Chrome task UAT; no demo clinical claims get live badges.

### P1 — Studio, media, spatial and device workflows

- [ ] Restore purpose-built Scene/Audio/Animation interaction while keeping
  POET composition rather than reproducing a nested DAW/DCC application.
- [ ] Bind transport, routing, meters, automation, scene graph, materials,
  lighting, cameras, animation, GIS, rendering, export, and media operations to
  real sessions/capabilities.
- [ ] Restore dataset video/CAD/super-resolution workflows with honest GPU/codec
  prerequisites and artifact provenance.
- [ ] Implement device discovery, pairing, role assignment, permission,
  workspace sync, display layout, remote control, disconnect, and receipts.
- [ ] Add task-level browser/hardware UAT.

### P2 — cross-platform application expectations

- [ ] Identity/profile/contact/group/role workflows with consent and provenance.
- [ ] Search that unifies COP, Semantic Library, conversations, connectors,
  agents, agreements, rights, datasets, and project entities with sensitivity.
- [ ] Activity center with filters, unread/read, actor, source, and backlinks.
- [ ] Import/export/recovery/checkpoint workflows with clear scope and receipts.
- [ ] Accessibility: keyboard completion, focus, labels, live-region restraint,
  contrast, reduced motion, and screen-reader task verification.
- [ ] Responsive layouts and usable empty/error/loading states.
- [ ] Final system-Chrome UAT covering actual end-to-end jobs.

### Parallel QApps-to-POET migration — do not perform as part of this list

- [Q] Inventory every legacy QApp route/manifest/source/user job.
- [Q] Publish machine-readable dispositions and fail CI on unmapped QApps.
- [Q] Convert each implemented QApp into native POET constructs/manifolds/
  containers/Library software entries with the original user job preserved.
- [Q] Refactor QApp Studio into native construct/manifold authoring.
- [Q] Add a bounded one-shot legacy importer where justified.
- [Q] Remove the legacy runtime only after replacements pass acceptance.

## 6. Exact next implementation recipe

Replies, mentions, read state, channel role administration, and non-destructive
moderation are implemented. Continue with **P0 bounded Social attachments
through the Semantic Library**.

1. Keep binary content out of COP records. Use the existing Library ingest API.
2. Add attachment authoring in a focused file; do not push
   `social_workspace.rs` over 500 lines.
3. Accept a bounded text/document source, URI, media type, sensitivity, and
   optional project/purpose tags. Reuse existing bounded file-reading patterns;
   do not claim binary PDF/OCR support when unavailable.
4. Ingest first. Only after successful ingest, add the canonical URI to a
   message as attachment metadata.
5. Backend must bound attachment count and require absolute URIs, media types,
   and sensitivity. Never store access tokens or binary bodies in a message.
6. Render attachments as labelled safe links/details; never render arbitrary
   attachment HTML.
7. Add blocked-relationship enforcement before accepting new direct requests/
   messages where an applicable block receipt exists.
8. Update this file and both trackers.
9. Run once:

```powershell
cargo check -p qualia-core-db --lib
cargo check -p poet --target wasm32-unknown-unknown
```

## 7. Definition of done for any item

Before changing an item to `[x]`, confirm:

- A named user can complete the whole intended job.
- The UI is domain-appropriate, not merely a generic form.
- State survives reload and has actor/time/provenance/sensitivity where relevant.
- Advertised actions call real implementations and expose exact failures.
- Authorization is enforced server-side for consequential operations.
- Derived state is computed from real records and handles malformed/conflicting data.
- No fabricated sample output is presented as live.
- Browser task UAT is recorded, or explicitly remains open.
- Trackers describe remaining gaps without broad family-level completion claims.

## 8. Known traps

- `decode.rs` is a pre-existing file over 1,000 lines. Keep lifecycle/control in
  focused modules and continue decomposition; do not grow it with services/UI.
- `social_lifecycle.rs` is close to 500 lines. New inbox/notification/moderation
  behavior belongs in separate files.
- `poet_record_api.rs` is a pre-existing large service. New family-specific
  validation should eventually be decomposed, but preserve atomic ledger checks.
- EventSource callbacks must retain the `EventSource`; otherwise streams close
  when the function returns. `native_daemon.rs` uses thread-local retained streams.
- A cancelled decode may emit partial text. Never persist it as a completed turn.
- Agent assertion approval is not permission to execute a connector/tool.
- Connector transport success is not semantic output validation.
- A Pulse publication notice is not proof of remote delivery or reading.
- Never auto-retry Cold, external, economic, write, or unknown-effect operations.
- Do not let a browser-only permission check substitute for daemon enforcement.
