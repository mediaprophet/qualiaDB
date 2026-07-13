# Workstream 4 — Assurance, Migration, Testing, and Release

**Goal:** make product claims evidence-backed, migrate useful WellFair data and behaviour
without importing old contradictions, and produce repeatable desktop/PWA releases.

## Scope

This workstream owns:

- audit traceability and maturity labels;
- legacy fixtures and migration;
- security, privacy, clinical, legal, financial, and accessibility gates;
- end-to-end and adversarial test harnesses;
- reproducible builds, installers, signed updates, backup/restore, and support diagnostics.

## Audit traceability

Maintain a machine-readable or table-based register with:

- audit capability ID;
- requirement statement;
- owning qApp/service;
- phase;
- status: Verified, Implemented, Prototype, Scaffold, Planned, Conflicting, Legacy;
- tests/evidence;
- network disclosure;
- policy and Sanctuary impact;
- open assurance review.

No feature becomes “Verified” because a UI control or document exists.

## Task packages

### A1. Legacy fixture corpus

Collect representative, redacted fixtures for:

- Samsung weight, heart rate, steps, and sleep;
- personal profile and disputed condition;
- medication, administration, substance, diet, and nutrition context;
- pathology, document, and extracted claims;
- mental-health observations and each candidate assessment;
- life events, cases, welfare, calendar, and boundaries;
- contacts, relationships, delegations, consents, guardians;
- Sanctuary records, commitments, tripwires, and evidence packages;
- communications events/transcripts/revisions;
- projects, contributions, shares, governance;
- finance, receipts, and credentials.

Fixtures include malformed, duplicate, conflicting, incomplete, and timezone-edge inputs.

### A2. Deterministic migration pipeline

For each source generation:

1. identify source schema/version;
2. hash source package;
3. parse without mutating the vault;
4. map to canonical record and evidence semantics;
5. present import plan;
6. commit accepted records through VaultService;
7. emit accepted/transformed/rejected/unresolved report;
8. support idempotent rerun.

Rules:

- never map generic “verified” without identifying its guarantee;
- never infer missing consent or authority;
- never import a fixed/demo PIN;
- owner explicitly selects Sanctuary-like content;
- preserve source timestamp/timezone uncertainty;
- preserve both sides of contradictions and disputes;
- raw attachments remain content-addressed with source provenance.

### A3. Security and privacy assurance

Threat-model:

- local malware and another OS user;
- stolen/unlocked device;
- malicious qApp/package;
- malicious paired mobile;
- hostile LAN and peer;
- replay/duplicate/oversized input;
- compromised external endpoint;
- coercion/duress observer;
- crash logs, telemetry, backups, and support bundles.

Required evidence:

- loopback/LAN separation;
- token scope/expiry/revocation;
- key at-rest and lifecycle review;
- qApp sandbox/path traversal tests;
- policy bypass suite;
- Sanctuary non-interference suite;
- wire fuzzing and payload caps;
- dependency/SBOM/license review;
- secure update and rollback.

### A4. Clinical and assessment assurance

Classify each output:

- descriptive;
- calculated metric;
- screening result;
- rule-based observation;
- hypothesis;
- professional assertion;
- prohibited diagnostic/advice claim.

Each assessment release requires:

- exact instrument/version;
- distribution licence;
- item text and response scale;
- scoring and missing-answer behaviour;
- interpretation thresholds and source;
- repeatability/history;
- locale/accessibility review;
- disclaimer and crisis/safety pathway where appropriate;
- test vectors independently reviewed.

Emotion recognition, rPPG, adrenal-fatigue patterns, and similar prototype inferences remain
disabled until separately validated and ethically reviewed.

### A5. Legal, finance, credential, and evidence wording

Review:

- consent/capacity/guardianship;
- emergency access;
- evidentiary presentation claims;
- geolocation and safety;
- tax/OFX/accounting wording;
- cooperative obligation/equity/cash-out meaning;
- issuer/schema/proof/status/revocation;
- external timestamp claims;
- already-disclosed-data limits of revocation.

Hashes and signatures are never described as automatically “legal-grade.”
Agent and analytics reviews also test for discriminatory, manipulative, adversarial, or
anti-human-rights behaviour and preserve the project's auditable conduct requirements.

### A6. Accessibility and human factors

Test:

- complete keyboard path;
- screen-reader labels and announcements;
- high contrast and colour independence;
- 200%/400% scale and reflow;
- reduced motion, especially Sanctuary;
- 44 px touch targets in companion;
- permission-denied alternatives;
- plain-language consent and evidence explanations;
- safe duress interaction without distinctive failure signals.

### A7. Automated test layers

#### Unit/property

- deterministic IDs and conversions;
- record lifecycle invariants;
- policy fail-closed and minimum projection;
- crash/replay;
- token/signature/credential tamper;
- package determinism;
- merge/idempotency;
- parser/decoder fuzzing.

#### Integration

- qApp isolation;
- VaultService/WAL/graph/Q42 equivalence;
- job persistence;
- paired gateway and revocation;
- PWA OPFS journal recovery;
- two-node partition/rejoin;
- external adapter mock failures.

#### End to end

Maintain one release-blocking journey:

```text
create vault
→ import Samsung
→ inspect provenance
→ add medication/diet
→ add sensitive record
→ pair companion
→ request/approve minimum projection
→ verify recipient receipt
→ revoke
→ verify subsequent denial
→ export package
→ restart offline
→ verify history and Sanctuary isolation
```

### A8. Build and release

Desktop pipeline:

1. pin Rust, Dioxus, Tauri, wasm-pack, and wasm-opt versions;
2. build canonical WASM profiles;
3. build Webizen Studio;
4. package desktop;
5. run installer smoke tests;
6. sign binaries and update metadata;
7. emit SBOM and checksums;
8. publish only after migration/recovery/E2E gates.

PWA pipeline:

1. build signed linked-companion WASM;
2. assemble deterministic package;
3. verify manifest/assets/service worker/CSP;
4. run secure-origin and offline browser tests;
5. sign archive and update metadata;
6. test rollback and prior-version migration.

Current helper scripts are inputs, not proof. In particular, replace the no-op mobile build
script and add an actual desktop release job.

### A9. Backup, recovery, and support

Deliver:

- encrypted backup with clear included/excluded domains;
- restore into a clean profile;
- key recovery policy;
- corrupt WAL/block quarantine;
- schema/package rollback;
- privacy-safe support bundle;
- explicit local wipe versus remote revocation semantics.

Sanctuary backup is separately selected, encrypted, and explained.

## Suggested agent split

| Agent | Ownership | Starts |
|---|---|---|
| A-A | fixtures and migration | record contract freeze |
| A-B | security/privacy harness | Phase 0 |
| A-C | clinical/assessment review assets | before relevant qApps |
| A-D | accessibility/E2E | shell alpha |
| A-E | build/release/SBOM | Phase 1 |
| A-F | backup/recovery/support | VaultService alpha |
| A-G | traceability/status register | immediately |

## Release evidence packet

Every release should contain:

- version and source commit;
- supported OS/browser matrix;
- package and WASM hashes;
- migrations performed;
- tests run and failures/waivers;
- known prototype/scaffold/deferred features;
- network adapters enabled;
- clinical/legal/credential assurance notes;
- security/privacy review date;
- rollback path.

## Completion criteria

- every shipped audit capability is traceable to code and executable evidence;
- legacy imports are deterministic, reviewable, and idempotent;
- security, Sanctuary, sync, accessibility, and recovery gates pass;
- installers/PWA packages are reproducible and signed;
- release notes do not overstate prototypes or evidence guarantees.
