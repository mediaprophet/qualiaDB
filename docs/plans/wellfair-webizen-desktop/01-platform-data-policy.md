# Workstream 1 — Platform, Data, Policy, and Storage

**Goal:** establish the single canonical backend that all WellFair qApps, desktop features,
the linked PWA, and the future native mobile peer use.

## Scope

This workstream owns:

- canonical WellFair record and vocabulary contracts;
- the `wellfare-core` domain adapter;
- VaultService transaction/recovery;
- PolicyService and enforcement middleware;
- identity/key/session lifecycle;
- provenance and typed receipts;
- durable jobs and projection APIs;
- the versioned host contract consumed by other workstreams.

It does not own qApp UI, PWA packaging, transport adapters, or legacy fixture migration.

## Existing code to retain

- `crates/wellfare-core/src/parser.rs`, `models.rs`, `rdf.rs`, `shapes.rs`
- `crates/qualia-core-db/src/q42/q42_volume.rs`
- `crates/qualia-core-db/src/wal.rs`
- `crates/qualia-core-db/src/services/daemon_graph.rs`
- `crates/qualia-core-db/src/governance/provenance.rs`
- `crates/qualia-core-db/src/modalities/logic/`
- `crates/qualia-core-db/src/identity/key_vault.rs`
- `crates/qualia-core-db/src/identity/credentials/`
- `crates/qualia-client-core/src/qapp_manifest.rs`
- `crates/qualia-client-core/src/qapp_api.rs`
- `crates/qualia-client-core/src/guardianship.rs`

## Code to retire or isolate

- `wellfare-core/src/qualia_bindings.rs::QualiaStore` as an authority;
- `wellfare-core/src/webizen.rs` as an independent policy VM;
- raw qApp calls to daemon query/storage endpoints;
- query authorization based on substring matching;
- non-expiring qApp tokens without audience, method, nonce, or revocation;
- time-derived/raw-file root key lifecycle;
- fail-open invitation, delegation, or relay verification in WellFair paths.

## Contract gate

The following types must be reviewed and frozen before parallel domain implementation:

```text
RecordId
RecordEnvelope
RecordLifecycle
EpistemicStatus
EvidenceType
SensitivityClass
ActorRef
AuthorityRef
PurposeRef
ConsentGrant
PolicyRequest
PolicyDecision
ProjectionSpec
VaultCommand
VaultEvent
Receipt
HostError
HostCapabilities
```

The wire representation is versioned independently from in-memory Rust types.

## Task packages

### P1. Canonical record vocabulary

**Primary ownership**

- `crates/wellfare-core/src/`
- new WellFair ontology/shape files under the repository's canonical ontology location
- compatibility mappings from legacy `wf:` terms

**Tasks**

- define record envelope predicates and lifecycle states;
- define actor versus owner versus author versus acting-on-behalf-of;
- define epistemic and evidence vocabularies;
- define asserted time, valid time, revision, predecessor, and tombstone semantics;
- define blob references for documents/media;
- define deterministic IDs and collision behaviour;
- add caller-buffered compilation to NQuins;
- add projection DTOs for qApps and standards exports;
- remove duplicate store/policy responsibility from `wellfare-core`.

**Audit coverage**

PRO-01..08, SEM-01..17, CLI-09..12, MHT-05, ACT-01..13.

**Acceptance**

- semantically equivalent source input compiles deterministically;
- round-trip projection preserves lifecycle, sensitivity, epistemic state, and provenance;
- malformed/ambiguous data fails or enters quarantine;
- compiler hot paths allocate no heap and obey NQuin ABI helpers.

### P2. Vault transaction coordinator

**Primary ownership**

- `crates/qualia-client-core/src/`
- narrow additions to `qualia-core-db` storage/WAL interfaces

**Tasks**

- implement `VaultService`;
- establish signed WAL event format;
- materialize successful events into daemon graph state;
- checkpoint to `.q42`;
- replay after restart;
- add outbox hook without coupling to a transport;
- ensure revision emission occurs only after durable WAL;
- define backup/snapshot and migration metadata.

**Acceptance**

- crash injection before/after each commit stage produces a valid recoverable state;
- replay is idempotent;
- graph state equals checkpoint/replay state;
- failed validation/policy creates no domain mutation;
- disk-full and corrupt-WAL outcomes are safe and user-visible.

### P3. Policy decision point

**Primary ownership**

- `crates/qualia-client-core/src/`
- `crates/qualia-core-db/src/governance/`
- `crates/qualia-core-db/src/modalities/logic/`

**Tasks**

- define a single `PolicyDecision` API;
- combine qApp capabilities, sensitivity, purpose, consent, credentials, delegation,
  guardianship, Sanctuary, and deontic rules;
- compile exact query/operation scopes;
- return minimum projection and obligations;
- emit typed decision receipts;
- add enforcement adapters for query, mutation, render, inference, export, sync, job, and
  device/network access.

**Audit coverage**

ACS-01..18, SAF-12/20, AGT-01/10/12/14, SYN-07/11.

**Acceptance**

- missing or unknown rules fail closed;
- expiry and revocation block subsequent actions;
- policy cannot be bypassed through an alternate daemon/qApp call;
- guardian-required actions suspend and resume through the bounded queue;
- reasons reference the norms/credentials/grants actually evaluated.

### P4. Identity and key lifecycle

**Primary ownership**

- `crates/qualia-core-db/src/identity/`
- `crates/qualia-client-core/src/social_connect.rs`

**Tasks**

- use OS credential/key protection on desktop;
- generate root/device/session keys using an OS CSPRNG;
- add unlock, lock, rotation, recovery, and revocation;
- issue short-lived qApp/device tokens with method, scope, purpose, audience, expiry, nonce,
  and revocation identifier;
- verify invites, delegations, relay messages, and credentials fail closed;
- separate owner, pairwise relationship, qApp, device, Sanctuary, and decoy keys;
- define credential trust/status cache and offline behaviour.

**Acceptance**

- no raw root secret is written unprotected;
- malformed/missing signature material is rejected;
- revoked or expired tokens fail on every route;
- key rotation preserves readable authorized history and blocks old future writes;
- Sanctuary and decoy keys are independent.

### P5. Audit and receipt service

**Primary ownership**

- `crates/qualia-client-core/src/`
- `crates/qualia-core-db/src/governance/provenance.rs`

**Tasks**

- define evidence taxonomy and receipt schema;
- chain import, mutation, decision, access, job, share, revoke, export, and sync events;
- support human-readable and CBOR-LD projections;
- exclude sensitive plaintext;
- represent external timestamp submission as pending/confirmed/failed;
- expose contest/dispute and resolution without rewriting history.

**Acceptance**

- each consequential command returns a stable receipt ID;
- the receipt states its guarantee and does not imply legal attestation;
- tampering or missing parents are detected;
- disputed facts retain original and contest history.

### P6. Durable jobs and capability registry

**Primary ownership**

- `crates/qualia-client-core/src/local_job_scheduler.rs`
- qApp/package capability code

**Tasks**

- persist queue state, retry, cancellation, trigger, and result receipt;
- enforce sensitivity and policy before enqueue and again before execute;
- add network disclosure descriptors;
- add resource ceilings and thermal/idle/charging/desktop-present triggers;
- isolate optional model/ontology/package capabilities.

**Acceptance**

- restart does not lose or duplicate jobs;
- revoked consent cancels ineligible queued work;
- Sanctuary payloads never enter ordinary jobs;
- UI can distinguish waiting, running, blocked, failed, cancelled, and completed.

### P7. `WebizenHostApi` v1

**Primary ownership**

- `crates/qualia-client-core`

**Tasks**

- define transport-neutral commands/events/errors;
- generate or hand-maintain conformance fixtures for Tauri, WSS, and WASM adapters;
- include capability negotiation and API/schema versions;
- avoid hard-coded ports in UI clients;
- expose only bounded projections and opaque handles to blobs/jobs.

**Acceptance**

- desktop qApps run through Tauri adapter;
- the same fixture suite passes against an in-process test adapter;
- future PWA adapter can be added without changing domain commands.

## Suggested agent split

| Agent | Ownership | Depends on |
|---|---|---|
| P-A | record vocabulary and `wellfare-core` compiler | ADRs 1–3 |
| P-B | VaultService/WAL/graph/checkpoint | record envelope |
| P-C | PolicyService/enforcement | record + identity contracts |
| P-D | identity/key/token lifecycle | security ADRs |
| P-E | receipts/provenance/jobs | event and evidence contracts |
| P-F | Host API and conformance fixtures | command/policy contracts |

P-A owns shared data types. Other agents consume them and do not create parallel versions.

## Verification command targets

At minimum:

```powershell
cargo test -p wellfare-core
cargo test -p qualia-client-core
cargo test -p qualia-core-db --lib
cargo check -p webizen-desktop
```

Add focused integration suites for crash/replay, policy bypass, and key/token lifecycle rather
than relying only on the full core test count.

## Completion criteria

- one durable write path;
- one policy decision path;
- one canonical record model;
- one qApp/host contract;
- one typed receipt model;
- no WellFair dependency on prototype stores or fail-open policy logic.
