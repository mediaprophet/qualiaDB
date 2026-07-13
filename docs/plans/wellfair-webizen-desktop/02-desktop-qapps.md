# Workstream 2 — Desktop Shell and Domain qApps

**Goal:** deliver a coherent desktop product from the audit's broad functionality without
creating qApp-specific stores, policy engines, or network paths.

## Scope

This workstream owns:

- WellFair desktop information architecture and shell;
- qApp manifests and UI;
- accessible forms, timelines, dashboards, and consent previews;
- domain-specific command/query usage through `WebizenHostApi`;
- human-readable receipts and assurance labels.

It does not own storage, policy authority, pairing transport, or cryptographic claims.

## UI architecture

The initial product is a WellFair workspace within Webizen Desktop:

- top-level owner/security/offline state;
- Personal, Health, Life, Relationships, Sanctuary, Projects, and Tools areas;
- context-sensitive privacy, epistemic, provenance, and sync indicators;
- one shared activity/receipt drawer;
- one shared consent prompt component;
- one shared external-network disclosure component;
- keyboard, screen-reader, high-contrast, text-scale, and reduced-motion support.

Each qApp declares:

- required shapes and record scopes;
- permitted commands;
- maximum sensitivity;
- device/filesystem/network capabilities;
- offline support;
- optional endpoints;
- required models/ontologies;
- desktop and linked-PWA surfaces;
- schema and host API versions.

## Task packages

### Q1. WellFair shell, onboarding, and operating state

**Audit IDs:** ENV-01..09, OPS-07/11/13/15.

Deliver:

- owner vault create/unlock/lock;
- no-network and offline status;
- accessibility preferences;
- capability/package readiness;
- storage/job/update status;
- explicit demo mode separated from real data;
- navigation and global receipt drawer.

Do not embed a second application state authority in Dioxus signals or browser storage.

### Q2. Personal Core

**Audit IDs:** PRO-01..08.

Deliver:

- identity/profile and administrative identifiers;
- language, ancestry, sex/gender expression, pronouns;
- emergency contacts;
- disability support and accessibility adjustments;
- conditions/allergies with supporting/refuting evidence;
- disputed/unconfirmed diagnosis state;
- dwelling, safety, hazards, violence, homelessness, and mobile shelter;
- record privacy/sensitivity control.

Sensitive fields need careful progressive disclosure and must not appear in screenshots,
telemetry, or generic search without policy approval.

### Q3. Social Book and guardianship

**Audit IDs:** ACT-01..13.

Deliver separate UI objects for:

- identity/contact;
- relationship;
- role and qualifications;
- delegation and legal basis;
- proxy consent;
- usage agreement;
- guardianship and selected data paths;
- project contributor relationship.

The UI must never imply that choosing “doctor,” “guardian,” or “family” alone grants access.

### Q4. Consent and selective disclosure

**Audit IDs:** ACS-01..18.

Deliver:

- access-profile request templates;
- exact recipient, purpose, data/field, operation, expiry, and redistribution controls;
- minimum-projection preview;
- per-section approve/deny;
- session TTL and termination;
- guardian threshold state;
- revocation and its limits;
- signed receipt view.

Emergency, credential, and project templates are requests evaluated by PolicyService.

### Q5. Health observations and sleep

**Audit IDs:** HLT-01..11, SLP-01..10.

Deliver:

- selected-folder Samsung import;
- import preview, source hashes, normalization report, duplicates/rejections;
- weight, heart rate, steps, sleep, and freshness views;
- quality, consistency, debt, trend, weekly comparison, heatmap, stage, and night detail;
- formula/explanation view;
- semantic export/query/validation;
- clear non-diagnostic language.

Exercise and cross-context sleep/safety inference follow after the main path is stable.

### Q6. Medication, substances, diet, and adherence

**Audit IDs:** MED-01..13.

Deliver:

- medication catalogue and cease-with-history;
- schedule and take/skip/overdue;
- local reminder permission flow;
- interaction warnings labelled by source/version/assurance;
- prescribed, OTC, supplement, alcohol, illicit, and other substance context;
- diet entry and daily totals;
- fasting, food insecurity, location, proxy, and psychological context.

External drug/food/barcode lookup is optional, disclosed, and disabled without consent.

### Q7. Clinical documents, pathology, and imaging

**Audit IDs:** CLI-01..13.

Deliver in order:

1. manual report/observation and attachment metadata;
2. timeline/trend and semantic export;
3. source/author and claim approval lifecycle;
4. real PDF/pathology parser behind confidence/review;
5. DICOM metadata and later volume integration.

Mock extraction is never presented as real parsing.

### Q8. Mental wellbeing

**Audit IDs:** MHT-01..16.

Deliver:

- observations, therapy, formulation, attachment;
- sensitive hypotheses linked to Sanctuary where appropriate;
- questionnaire record/history/export;
- approved DASS-21, BDI-II, PHQ-9, GAD-7, and K10 only after individual review.

Each instrument needs version, licence, exact items, scoring, interpretation, repeatability,
source, and prominent screening/not-diagnosis language.

### Q9. Life, welfare, cases, and boundaries

**Audit IDs:** LIF-01..17.

Deliver:

- life events, wellbeing impact, needs layer, recovery;
- supporting documents;
- cases, evidence, tasks, and unified timeline;
- assistance needs, welfare streams, and government letters;
- optional location with safe defaults;
- personal-priority calendar and mode;
- project conflict detection and explicit logged override.

### Q10. Sanctuary

**Audit IDs:** SAF-01..20.

Deliver only after the platform state machine and threat model:

- separate setup/unlock/lock;
- decoy state;
- sensitive note/hypothesis/contingency records;
- local commitment before storage;
- contingency contacts;
- tripwire/contradiction review;
- check-in scheduler;
- evidence/commitment export with typed guarantees.

Do not implement destructive PIN behaviour in this workstream.

### Q11. Credentials

**Audit IDs:** CRE-01..09.

Deliver:

- protected import/list/detail/delete;
- claim and verification-state inspection;
- presentation and QR;
- guardian-policy association;
- explicit distinction between JSON field selection and cryptographic selective disclosure.

### Q12. Communications

**Audit IDs:** COM-01..24.

Deliver in layers:

1. paired identity and caller gate;
2. signed usage agreement;
3. live section request/decision/receipt;
4. event chain and transcript/revision/package;
5. media and guest calls after transport/security validation;
6. transcription/translation tiers with service provenance.

### Q13. Cooperative projects and finance

**Audit IDs:** COP-01..19, FIN-01..17, SYN-01/02/07..11.

Deliver:

- projects, membership relationship, agreement;
- contributions and author chain;
- obligations derived from unique contributions;
- equity/stewardship and governance;
- project audit export;
- personal-boundary gate;
- ledger, balance, project links, receipts, tax context;
- CSV, tested OFX, and signed semantic export;
- sync status by operation state.

Payment rails and anonymous welfare flows are not part of the initial desktop release.

### Q14. Studies, rules, anatomy, and tooling

**Audit IDs:** RES-01..11, ANA-01..08, SEM tooling, OPS tooling.

These are follow-on qApps:

- paper/source/hypothesis and ruleset sandbox;
- signature-state-aware rule discovery;
- Webizen renderer-based anatomy with optional disclosed HRA lookup;
- Semantic Workbench, mapping, raw explorer, package diagnostics, and scoped cache controls.

Developer tools must not appear as ordinary user assurances.

## Shared component library

Create reusable components for:

- `SensitivityBadge`
- `EpistemicBadge`
- `EvidenceBadge`
- `ProvenanceTrail`
- `PolicyDecisionPanel`
- `ProjectionPreview`
- `ConsentGrantEditor`
- `ExternalDisclosurePrompt`
- `ReceiptViewer`
- `SyncState`
- `OfflineState`
- `SourceAttribution`
- `ValidationReport`
- `AssessmentDisclaimer`

These render backend decisions; they do not decide policy.

## Suggested parallel agents

| Agent | qApps | Start gate |
|---|---|---|
| Q-A | shell + Personal Core | Host API fixtures |
| Q-B | Social Book + Consent | identity/policy contracts |
| Q-C | Health + Sleep | record/import contract |
| Q-D | Medication + Nutrition | record/import contract |
| Q-E | Life/Welfare + Mental Wellbeing | Sanctuary/lifecycle contracts |
| Q-F | Sanctuary + Credentials | security/threat-model gate |
| Q-G | Communications | CompanionGateway contract |
| Q-H | Projects + Finance | signed operation/merge contract |

Avoid simultaneous edits to the central route/module registry. One integration owner adds
routes after each qApp exposes a stable module entrypoint.

## UI acceptance rules

- every write has clear save/pending/failure state;
- every shared view shows recipient, purpose, fields, expiry, and receipt;
- every external call shows what leaves the device before it happens;
- every inferred/extracted result shows source and epistemic status;
- every clinical/financial/legal score shows its limitation;
- revoked access is visibly distinct from deletion of already disclosed copies;
- offline state never silently falls back to a cloud service;
- Sanctuary can be exited/locked without leaving sensitive UI remnants;
- empty, loading, permission-denied, policy-denied, unavailable, and error states are distinct.

## Completion criteria

The desktop qApps pass the master plan's first usable journey entirely through the shared host
API, with no qApp-specific durable store, direct daemon access, or hidden network call.
