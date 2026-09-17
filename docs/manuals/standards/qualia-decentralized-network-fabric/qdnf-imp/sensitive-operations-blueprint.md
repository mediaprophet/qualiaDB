# Sensitive operations — implementation blueprint

**Date:** 2026-09-09

**Status:** Proposed implementation contracts and acceptance tests; no deployment certification.

**Parent:** [QDNF/QPR enhancement plan](0.0.37-enhancement-plan.md), especially E01–E03 and E12–E21.

This blueprint targets humanitarian and intergovernmental deployments, including confidential care and biometric processing for PEPs, VIPs, their children and other vulnerable people in hostile environments. Protection is selected from the information, purpose, capabilities and assessed threat; nationality, wealth or status alone does not establish trust or entitlement.

The profiles below are **Qualia deployment profiles**, not UN classifications or UN approval. Organisational classification mappings and operational acceptance must be supplied by the responsible authority. The [UN information-sensitivity record](https://digitallibrary.un.org/record/593127) is a policy reference to review with that authority, not a claim that this protocol implements every UN requirement. The [ICRC humanitarian data-protection handbook](https://www.icrc.org/en/data-protection-humanitarian-action-handbook) supplies relevant protection context.

## 1. Threat model and scope of guarantees

| Threat | Control to implement | Residual condition to report |
|---|---|---|
| Hostile local network or foreign carrier | Authenticated hybrid sessions, replay rejection, protected DNS-independent discovery, optional approved relays | An observer may still see timing, volume, device presence and access location |
| Malicious relay/storage provider | End-to-end recipient envelopes, authenticated manifests, opaque identifiers and minimal metadata | Availability and traffic correlation depend on topology and collusion assumptions |
| Stolen/seized endpoint | Protected key store, short-lived handles, auto-lock, minimal local history, sealed vault and prepared recovery | An unlocked/compromised endpoint or coercion can disclose accessible plaintext; remote wipe is not guaranteed |
| Compromised clinician/administrator | Scoped least privilege, separate key recipients, revocation, independent audit and compartment separation | Previously obtained plaintext cannot reliably be recalled |
| Abusive guardian/controlling partner | Independent confidential contact and reviewed mandate-specific access | Recovery/notifications must not expose the person to that controller |
| Forged authority or silent policy change | Exact signed artifacts, verified issuers, pinned interpretation and generation-bound permits | A compromised authorised issuer requires revocation and organisational response |
| Model/tool exfiltration | Information-flow-labelled execution, denied network/tool egress unless permitted, isolated context and output checks | A malicious endpoint or unqualified model runtime remains outside the guarantee |
| Biometric reuse or population linking | Local matching by default, purpose-bound templates, selective release and no public biometric index | Biometric characteristics are difficult to revoke; transformation alone is not proof of unlinkability |
| Prolonged disconnection/clock tampering | Signed bounded validity, monotonic clock checks, explicit freshness ceilings and ciphertext queues | Current revocation cannot be learned while disconnected |
| Harmful metadata/error disclosure | Restricted logging, indistinguishable public denial classes, minimised receipts and padded profiles | Timing/traffic non-interference needs separate measurement and has limits |

Do not implement covert retaliation, evidence destruction or automatic panic wiping. A duress feature requires a reviewed safety protocol; an apparently reassuring fake success response can endanger a user. Preserve a clear separation between confidential help, emergency access, recovery and ordinary guardian authority.

## 2. Independent data labels and protection profiles

A **data label** specifies who may use information and how. A **protection profile** specifies the endpoint/network controls required to handle it. Raising encryption strength alone cannot satisfy a missing recipient grant or compartment.

All profiles require the repaired authenticated cryptographic baseline. A lower-cost profile does not permit forged authority, plaintext private content or silent classical fallback.

| Profile | Typical use | Mandatory additions | Initial overhead/admission configuration |
|---|---|---|---|
| P0 — public authenticated | Published data and public service discovery | Authenticity, provenance, integrity, resource controls; explicit publication authority | No cover traffic; optional cache; no sensitive payload admitted |
| P1 — private ordinary | Routine known-peer personal exchanges | End-to-end confidentiality, private contacts, label inheritance, revocation, protected local keys | Padding to next 256-byte bucket; no mandatory relay; bounded history |
| P2 — sensitive professional | Medical records and authorised biometric workflows | P1 plus protected endpoint capability, scoped professional grants, sealed custody/audit, recipient-key visibility, export controls | Default 4 KiB application records; last-record padding; separate interactive/bulk budgets |
| P3 — hostile environment | Sensitive care through observed/hostile infrastructure | P2 plus approved relay/topology policy, restricted notifications/local retention, short capabilities, configurable batching/cover traffic, independent recovery | Target 4 KiB traffic cells; two approved relay hops where available; initial optional cover budget 4 KiB/s/direction for admitted active sessions |
| P4 — compartmented mission | Highest-consequence deployments under a mission authority | P3 controls selected or replaced by a reviewed isolated-bearer plan; dedicated compartment workers, controlled gateways, stronger endpoint assurance, witnessed administrative release | Dedicated cells and reserved control capacity; configurable traffic schedule and offline transfer; no universal latency promise |

These are starting **configurations**, not proven anonymity parameters. Padding records are fragmented into bearer-sized protected packets; a 4 KiB application record does not assume a 4 KiB MTU. Four KiB/s is approximately 14.1 MiB/hour in each direction before headers, relays and retransmission. Charge all of that at host/provider admission. Relays add trust assumptions, latency and bandwidth costs; one relay does not provide anonymity by itself.

P4 may select an isolated local bearer instead of remote relays. Profiles are capability sets with required predicates, not a simplistic integer ladder where every deployment must enable every feature above it. The label can require a minimum named profile plus additional controls. E16 must produce an audited compatibility table.

### Profile negotiation algorithm

1. Load the issuer-authenticated data label and local deployment policy.
2. Compute the required control set from label, task, sender policy and assessed environment.
3. Read peer capabilities from authenticated negotiation; validate any endpoint evidence using the configured verifier and freshness rules.
4. Intersect available configurations with every required control. Estimate byte/work/time overhead for each feasible configuration.
5. Select the least-cost feasible configuration according to explicit local preferences, then reserve its budget. A user's “extra protection” preference can strengthen it.
6. Bind selected profile, constraints and version digest into the session transcript and operation permit.
7. If no feasible configuration or budget exists, return ProfileUnavailable/BudgetUnavailable. Offer a prepared alternative route, queue or reduced *authorised data projection*; never silently lower protection.
8. Revalidate at path migration, recipient change and queued release. Send new plaintext only after a compatible configuration is established.

An organisational cost preference or paid tier cannot reduce required protection. Conversely, small devices need not instantiate expensive P3/P4 machinery to serve P0/P1 traffic. A remote network cell may carry ciphertext for a constrained endpoint without gaining decryption authority.

## 3. Bounded label contract

Implement `qdnf/policy_labels/` as a directory-backed library. Start with `types.rs`, `decode.rs`, `verify.rs`, `join.rs`, `flow.rs`, `release.rs`, `projection.rs` and separate tests. One owner controls its public types.

Use the existing StrongDigest and OperationId representations. The following is a proposed semantic schema, **not an assigned wire registry or a change to NQuin bit layout**.

| Field | Meaning and validation |
|---|---|
| schema_version / vocabulary_digest | Exact pinned label vocabulary. Unsupported required meaning fails closed |
| label_id / issuer / authority_instrument | Full digest references to exact bytes and scoped verified authority |
| confidentiality | Local C0 Public, C1 Private, C2 Sensitive, C3 Compartmented; Unknown is a separate failure state, never zero/Public by default |
| compartments | Sorted unique full-digest IDs identifying required clearances; maximum 16 per initial profile |
| audience_rule | Digest of verified recipient predicate/allow-list policy; not a public patient/clinician list |
| purpose_rules | Up to 16 sorted required policy references; all must hold |
| restrictions | Versioned prohibitions such as NoRedistribute, NoExternalAI, NoTraining, NoBiometricReuse, NoPublicIndex |
| required_controls | Up to 16 pinned control predicates such as protected endpoint, approved relay or independent release |
| validity / freshness | Valid-from, not-after, maximum accepted authority age and generation requirements; explicit timestamp units |
| retention_policy | Separate use/custody/operational/evidence retention predicates; no implicit permanent retention for higher sensitivity |
| release_authorities | Verified policy references identifying who can approve a particular relaxation, purpose and destination |
| origin/provenance | Referenced source envelope and derivation record; historical semantics remain reproducible |
| integrity/provenance status | Separate assessed provenance/trust state; confidentiality escalation must not promote untrusted input to trusted fact |

Initial decoded label budget: 4 KiB excluding streamed signatures and referenced artifacts. Initial evaluation job: at most 64 directly referenced label inputs, 16 compartments, 16 purpose clauses and 16 required control clauses in its accumulator. These are local admission limits to measure, not global graph limits. Overflow returns NeedContinuation or Capacity with no weaker partial label.

Use fixed caller buffers and checked lengths. Nested policy expressions are compiled to the existing bounded Webizen fragment; do not add recursive arbitrary expressions to the packet parser. Large source sets use an incremental accumulator and a streamed provenance commitment with counts, domain separation and original references retained through core ownership.

Wire representation uses pinned CBOR-LD semantics and a specified deterministic encoding/profile for newly issued signed artifacts. Verify signatures over exact original bytes, not over a locally “equivalent” reserialization. Exclude signature/self-digest fields from their own commitment through an explicit versioned preimage definition.

The core sensitivity byte can cache a conservative projection, but it cannot contain all compartments, purposes and releasability rules. It points to a validated label artifact through the existing semantic reference mechanism. If that reference is missing or invalid, protected processing stops. No new opcodes or field bits are allocated before the registry/ABI owner approves them.

### Local interfaces to implement

These are interface contracts for E13; use repository error and buffer conventions when finalising signatures:

| Operation | Inputs | Output and failure rules |
|---|---|---|
| decode_label_into | Original bytes, fixed LabelFields output, parsing budget | Untrusted fields; reject oversize, duplicate/conflicting fields and malformed lengths |
| verify_label | Untrusted fields, exact bytes, issuer/instrument proofs, current authority snapshot | Private VerifiedLabel handle bound to owner/generation; no boolean “signed” argument |
| join_labels_into | Request label, labels of all data/control dependencies, fixed accumulator | Verified derivation constraint or Capacity/Conflict/NeedContinuation; never omit a restrictive input |
| authorise_read | Principal/acting-capacity grant, object label, purpose, endpoint profile, current clock | Opaque ReadPermit or typed denial |
| authorise_egress | Output label, recipient, sink capability, current policy generations, requested action | Opaque EgressPermit binding object digest and target |
| release_derivation | Original label, proposed transformation/output, release instrument(s), reviewer decision | New released derivation and provenance; original remains unchanged |
| store_labelled_object | Label handle, content reference, transaction owner, resource lease | Atomic protected object reference; data cannot commit first and acquire its label later |

Every trusted handle is validated again at its consumption boundary. A valid Rust type from yesterday is not current authority after revocation or owner generation change.

## 4. Response inheritance and mandatory information flow

### Label join

For a response or derived artifact, start with the request/conversation handling constraints and every actual information dependency, including control dependencies such as a confidential record deciding whether the answer is “yes.”

1. Confidentiality becomes the maximum input level.
2. Required compartments and prohibitions become the union.
3. Audience, purpose and environment policies become conjunctions. For explicit allow-sets this is intersection; an empty feasible audience/purpose yields Conflict, not unrestricted access.
4. Required protection controls become the union; required assurance never decreases.
5. Use validity cannot outlive any applicable source/authority limit. Required freshness uses the strictest bound.
6. Retention constraints are evaluated separately for each allowed purpose. Conflicting minimum/maximum retention yields Conflict. Preservation holds suspend authorised deletion only through the separate hold owner; they do not grant new viewing rights.
7. Provenance accumulates all contributing sources. Low-integrity input stays identified; do not turn “most confidential” into “most trustworthy.”
8. Keep each restriction associated with its originating source and governing release policy. Release authority is not unioned into “any issuer may release everything.” Relaxing a combined output requires valid authority for every constraint being relaxed; unresolved ownership denies release.
9. Store the resulting label and derivation atomically with output. Revalidate policy generations at the ordered release boundary below.

### Host-owned dependency context

The runtime, not the application or model, owns a monotonically restrictive JobLabelContext. Before any protected read returns bytes, its storage/tool adapter verifies the label and joins it into that context. A failed join returns no bytes. Callers cannot supply an empty dependency list to bypass inheritance. Reads through shared memory, indexes and foreign-function interfaces require the same adapter boundary or an isolated worker already labelled for their entire admitted input set.

Every output builder, error builder and tool-egress adapter consumes the owner's current context. A forked/asynchronous job inherits it before dispatch; its replies merge back before parent output. No child can lower parent restrictions. Models with opaque internal dependency tracking receive the joined labels of their complete prompt, retrieval, tool and retained context inputs.

Streaming outputs carry the context at each release boundary. A later read can increase restrictions only on bytes not yet released; therefore tasks that may consult more-sensitive inputs must declare and admit a conservative upper-bound label before streaming, or buffer output until the dependency set is sealed. Context reuse requires a new generation and verified cleanup. Tests must attempt to omit a protected read from an application-supplied provenance list and still observe restricted answers, errors and tool calls.

### Revocation and release ordering

One authority/release owner serialises grant revocation with EgressPermit consumption. Checking a generation and later using a token in an unrelated owner is insufficient. Permit consumption atomically checks current grant/contact/profile generations, records the object/chunk and recipient, and commits a bounded release intent. If revocation wins this ordering, consumption fails.

Define that commit as the irrevocable authorisation point for only the recorded bounded chunk. The sink then performs that authorised release; a later revocation cannot retroactively withdraw bytes already committed to disclosure. It prevents subsequent chunks and new release intents. Keep the admitted chunk small, bound the time from intent to sink transfer, and cancel unconsumed intents. Do not issue a whole-session token that bypasses later revocations.

For restart, an unresolved consumed intent cannot be replayed blindly: reconcile sink acknowledgement and current authority, and require a new authorised retry where status is uncertain. If authority and sink are in separate processes, implement an explicit fenced owner/intent protocol with the same ordering; a timestamp check is not a substitute. Test barriers after validation, after token issuance, immediately before consume and after consume so the documented linearisation point is observable.

Never drop an input to make the join fit. A bounded continuation may accumulate more inputs without emitting output. Opaque external model results receive at least the full input/context restrictions because fine-grained dependency tracking cannot be assumed.

### Worked examples

| Input | Required default output |
|---|---|
| Patient sends a sensitive question to a known clinician | Clinician's answer retains sensitivity, care-purpose restriction and recipient constraints |
| Question includes a child's medical record and VIP-family association | Answer carries both compartments; no family association appears in routing/public discovery metadata |
| Two records have incompatible audience predicates | No combined disclosure; request an authorised separate projection or reviewed release |
| A model summarises a restricted record without naming the patient | Summary remains restricted; name removal is not proof of de-identification |
| Query returns zero matching medical records | Result/error timing and existence metadata are treated as sensitive where they reveal the underlying search |
| A protected conversation asks for a generic acknowledgement | Default acknowledgement inherits conversation handling; a public release requires the configured narrow release rule |
| Clinician prepares an explicitly approved public-health aggregate | New derivation may have relaxed handling only after the release policy verifies the transform, audience and risk; originals remain protected |
| An obligation is fully paid down | Recovery charge stops; data classification and privacy permissions stay unchanged |

### Enforcing sinks

The following are egress or materialisation sinks, not harmless implementation details:

- QPR replies, datagrams, file transfer, relays with plaintext access and LIG/API gateways.
- UI views, clipboard/export/print/screenshot facilities under application control, notifications and accessibility integrations.
- Search snippets, counts, sort order, pagination markers, cache keys and error messages.
- Agent tool calls, remote inference, prompts, embeddings, training data, model output and retrieval indexes.
- Logs, traces, crash dumps, financial receipts, evidence bundles, backups and removable media.

Every sink needs an explicit adapter and acceptance test. E13 must inventory them; a new sink fails build/review if it has no marking policy.

For agents, place each protected job in a labelled execution context with tool/network authority limited by that context. Pooled workers must clear/partition context, caches and temporary files before reuse. Prompt instructions are not an enforcement boundary: the host checks every attempted tool call and output even if the model ignores the label.

Recipients see a clear handling indicator and any required acknowledgement before opening protected material. Protocol receivers must preserve and enforce machine-readable markings. Visible watermarks may support handling but can themselves expose a patient's name; use policy-approved opaque references. The stack cannot prevent an authorised malicious recipient from photographing or manually copying plaintext.

## 5. Known-peer clinical workflow

Use existing contact/clinical primitives after repairing E01/E02/E12. Split the current 500-line clinical.rs into focused modules without changing the public module path.

### Pairing state machine

| State | Allowed input | Result |
|---|---|---|
| Unpaired | Private invitation with authenticated key/channel binding | PendingVerification; no clinical content |
| PendingVerification | Verified clinician scope, confirmed patient/representative authority, key confirmation | ActivePair with scoped relationship version |
| ActivePair | Current standing care grant and compatible profile | Permit selected medical operation |
| ActivePair | Suspension, revocation, disputed identity or key compromise | Suspended/Revoked; block new release |
| Suspended | Reviewed fresh verification and renewed grant | New ActivePair generation; queued old permits remain invalid |
| Revoked | Fresh pairing workflow | A new relationship, never silent reactivation |

`Request`, `Consent`, `Suspended` and `Blocked` are not interchangeable with active mutual contact. A signed invitation proves only what its issuer was authorised to assert.

### Send and reply sequence

1. Sender selects the clinician and the actual recipient key boundary. If a hospital service can decrypt, display that fact; do not imply only one named clinician can read.
2. Sender selects records/attachments and a standing or per-operation care permit. Confirm patient-record association separately from network identity.
3. Compile the execution permit and joined label; negotiate a feasible P2/P3/P4 configuration.
4. Allocate one unique operation ID from the durable owner. Encrypt the payload for its authorised recipient set using the qualified envelope/profile.
5. Commit ciphertext, label, manifest and outbound operation state atomically; transmit through QPR.
6. A custody node stores authentic ciphertext and returns a signed durable custody receipt only after actual persistence. It cannot decrypt or infer read status.
7. At recipient release, check actual current contact, audience, purpose, key generation, grant freshness and endpoint capability. Expired/offline-stale content stays sealed.
8. Decrypt and verify object/label binding. Record application acknowledgement separately from transport receipt. “Clinician reviewed” requires an explicit clinical application action.
9. Clinician's response enters the same label-join/authorisation/encryption path. It inherits the request and every consulted record by default.
10. On departure/revocation/key compromise, invalidate grants and queue permits. Previously disclosed bytes cannot be recalled; recovery creates new authority and keys.

Initial attachment plan: fixed-size verified chunks referenced by an authenticated manifest, streamed through the core volume owner. Per-chunk authentication binds object/manifest identity and index; final manifest/content checks prevent omission, reorder and truncation. Choose chunk size through MTU/latency/storage measurements; do not embed a magic large allocation in clinical code.

### Clinical interoperability

Use a version-pinned FHIR adapter and appropriate imaging/document adapters. Translate markings through a reviewed mapping of code-system/version/meaning; retain the full original label when the external format cannot express a constraint. Reject export if the destination cannot enforce required handling. FHIR labels connect data to an agreed policy framework; tags alone do not enforce it. See [HL7 FHIR security labels](https://hl7.org/fhir/security-labels.html).

Emergency access is a separate, pre-authorised policy workflow with bounded purpose, lifetime, recipient and protected audit. An “emergency” string or clinician role does not bypass the label. Where policy permits offline emergency access, prepare a narrowly scoped package in advance and explicitly record its freshness limitation; do not simulate current revocation knowledge.

Children's care uses the applicable capacity, guardianship, clinical confidentiality and safeguarding instruments. Protect independent confidential help. Do not derive blanket parental access from family membership or automatically disclose to every guardian.

## 6. Biometric lifecycle and advanced processing

Biometric data is not a reusable secret or an identity root. [NIST's authenticator guidance](https://pages.nist.gov/800-63-4/sp800-63b.html) distinguishes biometric activation from independent authenticator possession; the [ICRC biometric analysis](https://blogs.icrc.org/law-and-policy/2021/09/02/biometrics-humanitarian-delicate-balance/) addresses the particular risks of humanitarian biometric data. Implementation must not require a vulnerable person to surrender biometrics merely to obtain basic humanitarian access.

### Object types

| Object | Required metadata | Default handling |
|---|---|---|
| Raw capture | Modality, acquisition device/assurance, consent/mandate, timestamp uncertainty, quality, subject association | Most restrictive applicable label; local ephemeral processing unless retention is explicitly necessary |
| Derived template | Algorithm/version/parameters, source provenance, purpose/domain, transformation version | Encrypted, domain-bound, no public lookup or reuse |
| Match operation | Probe/reference references, authorised purpose, algorithm threshold, device/worker capability | Bounded confidential computation; reference scope must be authorised |
| Match result | Score/decision, uncertainty, quality/spoof indicators, provenance and expiry | Sensitive derived data; not an automatic authority grant |
| Identity/credential decision | Independent authorised issuer/instrument and reviewed evidence | Separate claim with scope, purpose and revocation; never implicit sameAs |

Raw face/voice/iris/fingerprint/gait/medical biometric modalities require separate schema and retention decisions. Do not collapse them into one generic biometric hash. A child’s changing characteristics and medical/accessibility conditions must be addressed by the application validation corpus and alternate workflow.

### Implementation stages

1. **Local activation baseline:** integrate platform key-store/authenticator APIs without exposing templates to QDNF. The network sees a scoped credential proof, not the biometric.
2. **Authorised remote matching:** selected recipient/worker receives only required encrypted artifacts under E13. Use a qualified algorithm implementation, fixed caller buffers and documented threshold/quality behaviour.
3. **Template protection:** evaluate a domain-specific cancellable transformation with tests for cross-domain linking and reconstruction. Rekeying an envelope does not revoke an already disclosed raw biometric.
4. **Advanced private matching:** choose one reviewed secure multiparty or homomorphic construction for a narrowly specified operation; reuse the existing privacy engine where compatible. Specify participants, collusion threshold, malicious/semi-honest assumptions, leakage, numeric encoding and failure behaviour.
5. **Qualification:** before selecting a remote-matching implementation, the biometric owner must approve a contract specifying modality/device/use-case, false-accept/reject and spoof thresholds, statistical confidence/sample-size requirements, relevant population/accessibility coverage, attack corpus, leakage/linkability limits and failure actions. Missing or failed criteria keep that capability unselectable. Compare to plaintext reference results without exposing the production corpus; measure error, cost, memory, leakage and side channels. Synthetic fixtures prove implementation correctness, not operational biometric accuracy. Do not claim full secure biometric recognition because encrypted vector arithmetic works.
6. **Revocation/recovery:** rotate domain templates where supported, isolate compromised references and offer an alternative identity/contact workflow. Never silently preserve a compromised biometric identifier as a network authority.

GPU/accelerator execution must charge buffers and prove isolation/cleanup. Confidential-compute attestation, where selected, is fresh evidence of a declared platform state; it is not proof of a bug-free application or protection from all side channels. Unsupported cryptographic features remain unavailable, not mocked.

## 7. Hostile environments, offline operation and compartment gateways

### Prepared mission package

Before disconnection, an authorised operator can provision a sealed package containing:

- Scoped peer/clinician invitations and verified key material, expiry and revocation snapshot age.
- Pinned contracts, ontologies, label mappings, necessary clinical projections and offline authority policy.
- Compatible profile configurations, trusted relay/contact routes and endpoint recovery instructions.
- Finite byte/work/energy budgets and, if applicable, bounded provider/payment allotments.
- Audit/evidence selection policy, retention rules and emergency contacts.

The package manifest binds every dependency and is verified before activation. Import is atomic; a partially imported package cannot grant authority. Do not label offline permissions “current” after their maximum freshness expires.

### Queue states

`Prepared → Sealed → CustodyAccepted → RecipientReachable → ReleaseAuthorised → Delivered → ApplicationAcknowledged`

Each transition has an owner and actual evidence. Revocation/expiry can move any pre-release state to `Held` or `Expired`. A receipt retry is idempotent. A crash restores the last durable state. Content/reference absence produces `MissingArtifact`, never Delivered.

### Traffic protection

Start with authenticated private links and end-to-end encryption. P3/P4 add only threat-modelled mechanisms:

- Opaque per-scope rotating contact/routing identifiers and private discovery. No plaintext diagnosis or family association in tags.
- Relay policy that separates observable roles where topology permits; disclose collusion assumptions.
- Fixed traffic classes, bounded batching and optional cover traffic charged before sending. Use unpredictable schedules only through qualified entropy; simulation uses injected deterministic entropy.
- Congestion-aware reduction of optional cover traffic only if the selected profile permits it. If cover is mandatory, pause sensitive release or select another compatible route rather than quietly dropping it.
- Separate tiny interactive messages from bulk imaging schedules; do not duplicate sensitive payloads onto unapproved paths to improve latency.
- Measure observer success under the stated adversary; byte padding alone does not prove resistance to timing correlation.

### Controlled transfer between compartments

Create `qdnf/gateways/compartment/` with import, policy-map, inspect, release, export and audit modules. A gateway is a distinct principal with explicit input/output compartments and no general decryption privilege.

1. Verify envelope, source label and recipient compartment authority.
2. Resolve a versioned semantic mapping; unknown/unmappable restrictions deny transfer.
3. Perform a permitted transformation in a labelled isolated worker.
4. Require the exact release instrument/review when relaxing constraints; P4 administrative release may require two independent authorised reviewers.
5. Bind output label and provenance to the transformed bytes; export atomically.
6. Record a protected evidence receipt without copying unnecessary raw payload into operational logs.

Two signatures from agents controlled by the same person are not independent approval. Do not simulate threshold cryptography by counting flags. Use separately verifiable signatures and a reviewed approval policy unless a qualified threshold scheme is explicitly selected.

## 8. Evidence, retention and operational safety

Use short-lived diagnostics by default and promote a justified subset into protected evidence storage. Capture selection must retain enough provenance and contrary context to avoid a misleading record. A selected log can support investigation without retaining every packet forever.

The evidence state machine is `Ephemeral → Selected → PreservationCommitted → Held → ReleasedForRetentionDecision → Deleted/Archived`. Holds are independent references; releasing one hold does not erase another. Every state change is durable and generation-bound.

| Requirement | Implementation direction |
|---|---|
| Original bytes | Preserve exact authenticated messages/artifacts and pinned interpretation dependencies, not only digests |
| Time | Record clock source, uncertainty and ordering; Lamport order is not a certified wall-clock timestamp |
| Custody | Record issuer, receiver, object digest, operation/generation and actual durable acknowledgement |
| Long-term verification | Retain historical key/authority material and verification results; renew protection/attestation without overwriting originals |
| Deletion | Enforce retention per purpose and holds through core GC; document backups/replicas and deletion acknowledgement limits |
| Disclosure | Export the authorised subset with full labels, examiner authority and redaction/provenance; unrelated medical data remains sealed |
| Failure | Show missing artifacts, uncertain time, incomplete chains or unavailable keys explicitly |

A remote-wipe request is best-effort and never a substitute for local encryption or minimal retention. Deleting a local key does not establish that a remote plaintext copy was erased. These are technical evidence facilities, not a promise of successful prosecution or a substitute for the relevant organisation's records policy.

## 9. Concrete acceptance tests

All test persons, biometrics and medical records are synthetic or explicitly approved test data. No real vulnerable-person corpus is required to begin implementation.

| ID | Arrange / action | Required result |
|---|---|---|
| S01 | Protected object arrives without its label artifact | No plaintext or success response; bounded MissingLabel result |
| S02 | Change confidentiality/purpose bytes without valid issuer signature | Verification fails before state advance |
| S03 | Sensitive request → public-looking model summary | Output inherits source/request restrictions |
| S04 | Two labels have disjoint authorised recipient sets | Join returns Conflict; no unrestricted empty-set interpretation |
| S05 | 17 compartments in a 16-slot accumulator | Capacity/continuation; no truncation or dropped compartment |
| S06 | Cached permit with old consent generation | Egress denied/re-evaluated; cache cannot authorise |
| S07 | Revoke clinician after enqueue; revocation wins before atomic release-intent consumption | Ciphertext may remain in custody; no new plaintext release |
| S08 | Set clinical contact to Suspended/Request/Blocked | No medical grant from “not blocked” alone |
| S09 | Send ordinary bytes labelled as ciphertext | Qualified envelope verification fails |
| S10 | Hospital changes receiving team keys | Sender-visible recipient boundary and fresh grant required |
| S11 | Child seeks permitted confidential help | No automatic disclosure/notification to a restricted guardian |
| S12 | User requests P3; only P1 available | ProfileUnavailable, no downgrade |
| S13 | Cover budget exhausted when cover is mandatory | Pause/compatible route; no silent traffic-pattern downgrade |
| S14 | Public network observer inspects metadata | No direct patient/diagnosis/PEP-family labels; correlation metrics separately reported |
| S15 | Two jobs reuse an agent worker across compartments | No prompt/cache/temp/output leakage; labels and key state reset |
| S16 | Restricted agent attempts external search/model/tool call | Host blocks egress unless explicitly authorised |
| S17 | Embedding/vector index created from restricted records | Index and retrieval responses inherit restrictions; no public similarity oracle |
| S18 | Biometric match succeeds without an authority instrument | No session/identity privilege granted |
| S19 | Compromised template reused in a different purpose domain | Reuse rejected; no cross-domain identity merge |
| S20 | User cannot/will not provide biometrics | Available authorised alternative workflow remains usable |
| S21 | Offline package exceeds grant freshness or clock rollback detected | No pretend-current authority; explicit sealed/expired outcome |
| S22 | Process killed after custody bytes write, before receipt commit | Restart recovers consistent state; no false Delivered |
| S23 | Two preservation holds; release one while GC runs | Object remains preserved under the other |
| S24 | Paid-down commons obligation plus restricted medical label | No recovery surcharge; privacy still enforced |
| S25 | Gateway lacks a mapping for one required label | Transfer denied; no dropped restriction |
| S26 | Two approvals share the same controlling person | Independence policy rejects administrative release |
| S27 | Public error/count reveals private query membership | Protected or uniform external result under configured policy |
| S28 | New path completion carries a stale generation | No activation or release on the reused slot |
| S29 | Restored backup has old recipient keys/permissions | Revalidate current policy before release; old labels remain intact |
| S30 | Independent verifier receives a selected evidence bundle | Verifies originals/provenance or reports precise gaps |
| S31 | Labelled FHIR export drops a required caveat | Export fails or preserves it through an agreed extension/envelope; never silently succeeds |
| S32 | A user edits a visible marking or asks the model to ignore it | Machine enforcement and signed label remain authoritative |
| S33 | A release authority approves one redacted output | Only that output/audience/purpose is released; original unchanged |
| S34 | Worker crashes during biometric/GPU processing | Charged buffers reclaimed/zeroised where applicable; no retained unowned artifact |
| S35 | All permitted routes unavailable during emergency care | Clear unavailable/offline-safe outcome and prepared alternatives; no unknown-recipient fallback |
| S36 | Combined output uses sources A/B; only A's authority approves relaxation | No relaxation of B's constraints; combined release denied unless every affected policy is satisfied |
| S37 | Public request secretly reads sensitive data and submits an empty dependency list | Host context restricts answer, error and tool egress despite the omitted list |
| S38 | Revocation wins immediately after permit issuance but before atomic consumption | No release; every unconsumed chunk requires current authority |
| S39 | Revocation follows one committed chunk release intent | Only that bounded authorised chunk may complete; no subsequent chunk is released |
| S40 | Biometric accuracy/leakage thresholds are missing or a measured threshold fails | Remote matching capability remains unselectable; alternate authorised workflow is available |

Property tests for label join must establish commutativity, associativity (within admissible capacity), idempotence and monotonic restriction. Denial at capacity is acceptable; relaxing a label is not. Treat timing and covert-channel properties as separately bounded claims, not consequences automatically proven by those algebraic tests.

## 10. Junior developer and agent execution recipes

The intended implementer is a junior developer assisted by an agent approximately as capable as GPT-5.5. Reduce discretion at security boundaries: use the specified states, invariants and test oracles; ask the owning reviewer to resolve ambiguous protocol semantics rather than inventing cryptography or interpreting a success flag as proof.

### Recipe A — every child assignment

1. Read the owning module, parent E-package, applicable original checklist, repository instructions and relevant wire/semantic specification.
2. Record exact write paths, input/output types, predecessor revision and the one behaviour being delivered. If the task spans more than one lifecycle, split it.
3. Write the failing acceptance case through the narrowest public boundary that demonstrates the defect. State why it fails now.
4. Implement the smallest complete behaviour, including failure/cancel/expiry cases. Keep untrusted DTOs separate from verified handles.
5. Run formatting/type checks and the focused tests. Run allocation/race/fuzz checks when the task touches those boundaries.
6. Exercise integration through an actual caller; a new helper with no production caller does not finish an enforcement task.
7. Have a distinct reviewer attempt the negative cases and inspect the trust/memory boundary.
8. Hand off revision, diff, tests actually run, artifacts, limitations and the next dependent task. Check only the child requirement proved.

### Recipe B — join_labels_into

**Files:** types.rs, join.rs, tests/join.rs. **Predecessor:** reviewed schema and VerifiedLabel API.

- Caller provides accumulator storage and verified source handles. Validate every handle before reading constraints.
- Merge sorted compartment/restriction sets with a two-pointer loop into caller scratch; validate required capacity before committing.
- Keep audience/purpose policies as a conjunction of verified references; only intersect explicit sets when their semantics are pinned.
- Compute confidentiality maximum, control union and validity/freshness constraints using checked arithmetic.
- Preserve existing accumulator on any error; emit no derived object while inputs remain unprocessed.
- Append every input to the provenance commitment through the core owner.
- Test S03–S06 and algebraic properties with an independent slow test oracle using bounded generated cases.
- Integration: clinician reply and an agent-generated summary must both use this function through the common output builder.

### Recipe C — protected receive

**Files:** session/packet_protection/receive.rs, replay.rs, tests/receive.rs. **Predecessors:** E02 reviewed key schedule and verified session owner.

- Parse only the bounded header needed to locate a candidate session/key; treat packet number and routing metadata as unauthenticated.
- Check admission for scratch before decryption; reject unsupported versions/lengths without allocating.
- Derive the nonce from the qualified directional key/epoch and packet-number rules.
- Verify AEAD over exact associated data. No receive-window, key-retirement, delivery or financial state changes before authentication.
- On success, check replay/epoch and validate inner frame limits; atomically accept receive state according to the specified transaction ordering.
- Dispatch through current policy/label enforcement. Return caller-owned plaintext only to an authorised sink.
- Zero temporary plaintext on failure where present. Account for duplicate/decryption work and rate limits.
- Test tamper, replay, forged very high packet number, stale key, malformed authenticated frame and output-buffer shortage.

### Recipe D — real block verification and commit

**Files:** replication/source.rs, block_verify.rs, commit_adapter.rs, tests/recovery.rs. **Predecessor:** E06 storage transaction contract.

- Open an immutable source generation and authenticated manifest through the core API.
- Read actual bytes into a leased slice; enforce declared range and exact expected length.
- Verify digest/proof against the manifest's source/controller authority, not a caller-provided “verified” bit.
- Stage verified object, mutation and receipt under one operation identity.
- Commit through the core transaction owner; return a durable stage only after its durability contract completes.
- Recover by streaming the durable log/manifest; deduplicate replay by operation identity and effect digest.
- Test wrong bytes, wrong range, wrong generation, omitted block, repeated operation and process termination at each commit boundary.

### Recipe E — clinical release after waiting

**Files:** clinical/pairing.rs, permit.rs, mailbox.rs, release.rs, tests/queued_release.rs. **Predecessors:** E11–E14 interfaces.

- Read durable ciphertext and its exact label/manifest; a metadata-only mailbox item cannot proceed.
- Load current contact and authority generations from the verified owner.
- Confirm active pair, actual clinician/audience, purpose, selected projection, time/freshness and negotiated endpoint/profile.
- Recompile/revalidate when generations differ; do not ask a caller for grant_current=true.
- Obtain a single-use release capability bound to object/chunk digest, recipient and operation.
- Consume it through the authority/release owner's atomic ordering with revocation; decrypt/transfer only the committed bounded chunk through the protected sink and record actual acknowledgement.
- On revocation, keep sealed or expire according to custody policy; send no informative denial to unauthorised third parties.
- Test S07–S11, S21–S22, S29 and S38–S39, including deterministic race barriers.

### Recipe F — profile overhead without downgrade

**Files:** profiles/catalog.rs, requirements.rs, negotiate.rs, budget.rs, tests/negotiate.rs.

- Store named immutable control sets and their policy/registry digests.
- Evaluate requirements independently of user budget preferences.
- Enumerate only bounded candidate configurations. Reject any missing control before cost ranking.
- Charge padding, relay copies, handshakes, cover traffic, retries, storage and cryptographic work separately.
- Reserve resources through E03; bind selected configuration to authenticated negotiation.
- On failure, expose a typed reason and authorised alternatives without disclosing sensitive label details.
- Test S12–S14 with genuine byte counts in the deterministic harness and later the live bearer.

### Recipe G — sophisticated features

Use the [advanced algorithm recipes](advanced-algorithm-recipes.md) for the supplied QSR, handover, routing, transport, financial and evidence mini-specifications. The original versioned protocol suite defines their wire/semantic contracts.

For multipath, QSR handover, private biometrics, group encryption and compartment release, a junior/agent pair must receive a reviewed mini-spec before implementation. Its required contents are:

- Exact threat/failure assumptions and supported subset.
- Complete state table with owner, input validation, outputs and durable boundary for every transition.
- Versioned message/commitment definitions, maximum lengths, encoding and error behaviour.
- Memory/work/crypto budgets and continuation semantics.
- Independent reference algorithm/test vectors and negative cases.
- A real integration caller and an explicit qualification gate.

The parent plan already assigns these responsibilities. A “future verifier,” TODO body, unconditional success or mock boundary does not satisfy the mini-spec. When no reviewed construction meets the chosen threat model, leave that capability unselectable and implement the qualified supported profile; do not improvise a security guarantee.

## 11. Deployment qualification packet

Before any sensitive deployment, produce:

- The exact supported profile/capability matrix and organisational label mapping.
- Threat model, endpoint/key-store assumptions, dependency/crypto versions and independent review results.
- Results for S01–S40 plus relevant main-plan tests, with unsupported cases explicitly excluded from advertised capability.
- Measured latency, bandwidth, energy, resident/pinned memory and failure behaviour for actual devices/bearers.
- Credential onboarding, clinician departure, guardian dispute, device loss, revocation, evidence handling and incident runbooks.
- Operator training and visible user explanations of recipient access, offline limitations and delivery status.
- A deployment authority's acceptance of the selected configuration and residual risks.

Labels provide enforceable policy inside qualified endpoints and cooperating services. They cannot compel a compromised or malicious recipient to honour restrictions after disclosure. The system should make that boundary visible while minimising unnecessary disclosure in the first place.
