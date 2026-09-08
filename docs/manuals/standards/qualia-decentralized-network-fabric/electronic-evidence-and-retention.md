# Electronic Evidence and Retention

**Status:** Proposed semantic lifecycle and acceptance requirements; not an implemented evidence service

**Date:** 2026-09-06

## 1. Purpose and limits

Preserve selected electronic evidence with enough context for independent examination, while allowing
ordinary network diagnostics to expire. This supports incident investigation, dispute resolution,
defence and prosecution preparation. It does not establish admissibility, guilt or a legal outcome.
Applicable jurisdiction, purpose, authority, retention/review rules and disclosure constraints belong
to a versioned deployment policy maintained by the responsible organization; this design invents no
legal retention periods. Conflicting preservation/deletion requirements require an authorized decision.

NIST IR 8387 (2022), especially section 3.2, addresses source/custody documentation, protected hashes,
storage, retention and later readability. NIST SP 800-86 provides an incident-response forensic
process and describes timestamp uncertainty; it is not a substitute for legal procedures.
These inform this design, not a claim of certification or legal compliance.
([NIST IR 8387](https://doi.org/10.6028/NIST.IR.8387),
[NIST SP 800-86](https://doi.org/10.6028/NIST.SP.800-86))

The requirements below define meanings and transitions. Class names are illustrative, not allocated
ontology/registry identifiers, frozen wire records or a requirement to adopt one container layout.

## 2. Retention tiers and selection

| Tier | Intended information | Retention meaning |
|---|---|---|
| Transient execution | Packet buffers, parser scratch and online cryptographic state | Release at bounded operation/session completion; not automatically recorded as evidence |
| Routine diagnostics | Minimal counters and selected event details | Short configured TTL, for example 5 minutes |
| Incident context | Authorized diagnostic context around a trigger | Separate TTL, for example 1 hour; trigger does not grant unrestricted capture |
| Extended diagnostics | Specifically enabled troubleshooting traces | Another explicit TTL, for example 24 hours, with stricter scope/storage admission |
| Operational audit | Grants, decisions, changes, receipts, custody and failure/gap records | Policy-defined window and dependencies; pending obligations can prevent deletion |
| Selected case bundle | Selected originals plus their protected context and custody history | Durable preservation under case policy and review; ordinary diagnostic TTL no longer applies |

Diagnostic durations are illustrative operational settings, not legal periods or universal defaults.
Every record carries its selected tier, policy version, purpose, scope, capture origin, retention
basis and expiry/review basis. Compute expiry with explicit clock provenance; an uncertain clock or
restart must not silently accelerate deletion. Expose the coverage and durability actually available.

Selection is a recorded, reviewable relevance decision, not a filter for guilt. Retain relevant
negative results, failed checks, alternative explanations and potentially exculpatory material,
including evidence that undermines the selecting actor's hypothesis. Record inclusion/exclusion
criteria, time/source coverage, query/rule/software versions, overrides and known collection gaps.
Do not retain excluded private content merely to describe an exclusion; preserve the permitted
selection metadata and make the resulting limits visible.

Selective retention can miss evidence needed by a future case. Once unselected source data expires,
the system has no hindsight recovery capability. A digest, aggregate, graph projection or deletion
receipt cannot reconstruct the lost logs. Policy owners must accept and communicate that trade-off.

## 3. Preservation holds and atomic promotion

A PreservationHold identifies its issuer, verified authority, purpose, selectors, effective time,
review/release conditions and protected scope. A hold is an overlay across tiers: once accepted it
blocks TTL deletion, compaction loss, key destruction and conflicting retention changes for the
protected dependency closure. A review date is not an automatic release. Multiple holds are tracked
independently; releasing one does not cancel another or discharge an outstanding obligation.

Promotion and garbage collection share the core transaction owner:

1. Before expiry/deletion commits, resolve the selected objects and atomically install a durable hold
   or promotion pin against their current generations. GC rechecks this state at its deletion commit;
   an earlier scan is insufficient. If deletion already won, report the gap rather than accepting a
   fictitious preservation claim. Expired but still present bytes require authority to retain.
2. Protect the dependency closure through bounded iterative traversal. Until closure is known,
   pin the bounded committed snapshot/segments containing potentially reachable local dependencies,
   or use GC that preserves their reachability from pinned roots. Pinning only root bytes is
   insufficient. Include dictionaries, proofs, historical authorization, software and
   other required context. Cycles are visited once. If work exceeds a pass, checkpoint the protected
   frontier and continue; never declare a truncated closure complete.
3. Reserve destination capacity, recovery headroom and verification work. Write an immutable candidate
   bundle/manifest, verify reconstructability and required copies, and synchronize storage according
   to the promised durability class. Byte-preserving references can replace copies only when their
   retention, access and recovery guarantees satisfy the same claim.
4. Atomically publish the bundle, closure references, custody/promotion receipt and retention state.
   Remove temporary pins only after durable ownership transfers. Crash recovery preserves pending
   pins and resumes or quarantines incomplete promotion; it never exposes a partial bundle as complete.

Holds arriving while discovery is incomplete also protect matching future arrivals within their
authorized scope. If a selector cannot yet be safely resolved, quarantine the bounded affected
segment/scope from GC and admit resolution work; avoid silently treating it as an empty selection.
Unavailable dependencies remain explicit gaps. A missing dependency cannot be recreated by a hold.
For remote dependencies, obtain a verified preservation acknowledgment or protected local copy;
a local pin does not establish remote retention.

## 4. Bundle context and provenance

The protected closure includes what is needed to interpret and challenge the selected evidence:

| Component | Required meaning |
|---|---|
| Originals | Exact bytes as acquired, source locator/instrument, acquisition extent and method; distinguish original acquisition from an assertion about an earlier source |
| Acquisition record | Collector, authority/purpose, device/tool version and configuration, start/end observations, transfer path, errors and gaps |
| Proofs and historical authority | Original signatures, verification keys and available controller/delegation/revocation evidence, as-of observations and trust policy; preserve invalid and unverifiable results |
| Semantic dependencies | Exact contexts, lexicons, ontologies, contracts, shapes/rules and their digests/versions; bounded reference closure, not mutable future web lookups |
| Selection and analysis | Selection rules, software/build/model versions where relevant, parameters, queries, interpretations, uncertainty, contrary material and analyst amendments |
| Time evidence | Original timestamps/time zones, monotonic readings and boot epochs, clock source, calibration/synchronization observations, offset/drift estimates and uncertainty |
| Custody and protection | Hold/promotion events, storage and verification history, access, transfers, exports, redactions, key-management actions and authorized disposition |

An acquisition hash proves correspondence to the acquired bytes, not that an upstream producer told
the truth. Separate producer-asserted time, local observation, acquisition time and any independently
witnessed checkpoint time. Preserve raw readings when normalizing timelines; logical clocks order
specified events but do not supply calibrated wall time. Missing calibration stays unknown.

Apply the [identifier fabric's separate planes](./identifier-fabric-integration.md): NaturalAgent,
AI agent, organization and machine remain distinct; an asserted actor, claim, handle and cryptographic
instrument are not interchangeable. A signature authenticates according to its verified instrument
and scope; it does not by itself establish which natural person acted. Record role grants, reliance,
delegation and counterevidence over time without turning accusations into permanent person types.
Source: the consultation brief at local Git revision `6601dc811468d266f4ac952243ce65fb10372606`,
`docs/work-in-progress/IDENTIFIER_FABRIC_CONSULTATION_BRIEF.md`, especially sections 2c-3; the historical
file was read from the Git object and is absent from this checkout.

Derived reports, normalized exports and redactions are separately identified artifacts linked to
their inputs, transformation method, operator and authority. Never overwrite the acquired original
or reuse its signature/digest for altered bytes. A disclosure bundle can intentionally omit protected
content while declaring its disclosure scope; the protected originals stay under their own policy.

## 5. Integrity, custody and independent observation

Record each custody action with the actor/instrument, authority, object/bundle identities, source and
destination custodians, time evidence, verification result and outcome. A sender's transfer claim and
a receiver's acknowledgment are separate observations. Missing receipts are gaps, not inferred success.
Where independent witnessing is authorized, send a minimal scoped commitment to a separately governed
custodian and retain its verifiable receipt/checkpoint, privacy constraints and availability limits.

A local hash chain alone permits undetected suffix deletion or replacement of both history and head
when the verifier has no independently retained expected checkpoint. For example, presenting A-B
after deleting C from A-B-C still leaves valid links. A separately retained commitment to C reveals
the mismatch; it does not restore C's bytes. Witness collusion, unwitnessed intervals and selective
capture remain limitations. No universal public or permanent ledger is required.

Report byte integrity, instrument authentication, custody continuity, selected-set coverage and
substantive interpretation separately. A valid hash/signature/chain proves neither truth nor global
completeness. Even a complete selected bundle may omit facts outside its capture/selection scope.
Independent review must be able to inspect uncertainty and challenge the interpretation; automated
prosecution or a guaranteed adjudicative outcome is outside this service.

## 6. Storage, large artifacts and keys

Use existing core artifact, range, segment and transaction owners. A large forensic image or media
capture may span many immutable chunks/files with exact lengths, ordered ranges and full source/chunk
commitments in a bounded manifest hierarchy. Q42 holds semantic projections; a candidate
[QNF manifest/container](../qnf-network-container-draft.md) can package references and supporting
objects. Its illustrative 1 GiB cap is not a limit on a case or a reason to truncate an original.
Arbitrary evidence blobs need an explicit core storage adapter, not conversion into lossy Quin fields.

Use scope-safe deduplication only when sharing cannot reveal private membership or weaken separate
retention/access obligations. Each bundle retains logical provenance and references; physical sharing
does not collapse custody claims. Lossless compression must reproduce original bytes and digests;
retain codec/decoder parameters, test restoration and enforce expansion/work limits. Summaries,
embeddings, redactions and lossy media conversions cannot replace originals. Keep the hash-reference
graph acyclic: seal source artifacts first and join their identities in later manifests/commit roots.

Evidence-at-rest keys have their own custody, recovery, rotation and release policy. Holds protect
needed decryption keys as well as bytes; key deletion is disposition, not an accounting shortcut.
This does not retain long-term transport session keys or weaken forward secrecy. Store only lawfully
collected source material, with plaintext or encrypted form explicitly identified; unavailable past
session plaintext remains unavailable. Record any authorized decryption as a derived acquisition.

## 7. Provider roles, economics and capacity

An agent enabling a [NetworkProvider role](./semantic-network-roles.md) may separately accept an
evidence-custody grant. Routing, storage, reading, decryption and export are different permissions.
Custody grants no automatic access or export to another provider, investigator or public service.
Provide authorized parties with intelligible collector/provider identities, purposes, selection and
retention policies, transformations and custody history; record permitted restrictions on disclosure.
Actor transparency preserves the chain of responsibility without inventing a universal identity.

Custody economics bind the parties, stored scope, duration/review terms, copies/failure domains,
verification and retrieval service, migration, deletion authority and the exact durability claim.
Meter bytes and storage duration alongside energy, elapsed time and explicitly defined compute work.
Distinguish quoted, measured, estimated and unavailable quantities; payment or a receipt alone proves
neither persistence nor evidence authenticity. Expiring payment does not silently cancel a hold:
use funded reserve, renegotiation or an authorized, verified transfer with the custody chain intact.

The [network cell and parent governor](./network-cell.md) reserve memory, disk, verification work,
energy and recovery/repair headroom before accepting durability obligations. A hold cannot create
space, and pressure cannot silently evict protected evidence. Quarantine affected state, report
insufficient capacity and pause actions whose mandatory audit/preservation cannot be met. Maintain
bounded control, revocation and recovery traffic using reserved capacity; unaffected authorized work
can continue. Record actual loss/corruption and reconciliation needs rather than claiming protection.

Illustrative capacity planning, assuming an approved operational policy (not a legal schedule):
4 MiB/min of diagnostics for 60 minutes = 240 MiB; 8 MiB/day of audit for 30 days = 240 MiB;
three selected 256 MiB bundles = 768 MiB. Total uncompressed primary data = 1,248 MiB. Two independent
copies = 2,496 MiB; an assumed 64 MiB total index/receipt allowance plus 512 MiB recovery reserve gives
3,072 MiB (3 GiB) provisioned across both locations. No compression/dedup savings are assumed; measure
actual indexes, proofs, closure dependencies and promotion peaks. Growing holds require new admission
capacity. Two copies on one failing device do not satisfy the example's independence assumption.

## 8. Release, recovery and implementation evidence

Long-term preservation includes scheduled fixity/restoration checks, media/format migration and
cryptographic renewal before relied-on mechanisms become unsuitable. Preserve originals, old proofs,
historical validation material and migration/renewal records. New signatures attest their actual
claim and time; they cannot retroactively cure compromise or prove unobserved earlier existence.
[RFC 4998](https://www.rfc-editor.org/rfc/rfc4998.html) provides prior art for archive timestamps,
reduced hash-tree proofs and renewal. An adopted profile must specify trusted time/witness assumptions
and verification; QDNF does not claim ERS conformance. Retain proof paths needed by preserved objects
when unrelated objects expire; membership alone does not prove original collection completeness.

An offline examination bundle includes a human-readable inventory, selection/provenance narrative,
clock uncertainties, known gaps, verification results and bounded reproducible verification tools
or specifications. Explanatory reports and translations are linked derivations. A reviewer can trace
assertions to instruments, source bytes and context without trusting a running QDNF node.

Disposition requires all applicable holds released, retention obligations satisfied, no protected
closure reference and an authorized deletion decision serialized with GC. Record the policy,
authority, objects/replicas affected, observed outcome and exceptions. Address backups, exports and
key copies through their governed lifecycles; a local deletion receipt does not prove every remote
copy is gone. Keep required disposition/custody records under their own scoped retention policy.

Existing owners reviewed for reuse, without claiming the new lifecycle is implemented:

| Owner | Existing behavior and remaining boundary |
|---|---|
| [Sanctuary audit DAG](../../../../crates/qualia-core-db/src/crypto/sanctuary_audit_dag.rs) | Chained sealed records and archive/triage routing; actor DID is explicitly asserted, and chain validation accepts a valid prefix. No preservation-hold/GC transaction follows from these helpers. |
| [Core WAL](../../../../crates/qualia-core-db/src/wal.rs) | Native synchronous Quin append and DAG checkpoints; recovery materializes the log, and WASM methods do not establish durable persistence. Needs bounded recovery and multi-artifact preservation transactions. |
| [Volume manifests](../../../../crates/qualia-core-db/src/q42/volume/manifest.rs), [publication](../../../../crates/qualia-core-db/src/q42/volume/publish.rs) | Multi-segment metadata, range access and root replacement patterns; these do not yet guarantee arbitrary evidence-blob custody, hold-aware GC or crash-atomic bundle promotion. |

[P20: Electronic evidence lifecycle](./implementation-conformance.md#qdnf-p20--electronic-evidence-lifecycle)
must verify hold-versus-expiry races; bounded closure expansion and missing dependencies; crash-safe
promotion/GC/recovery; balanced selection and declared gaps; lossless restoration and redaction
lineage; historical authority/clock uncertainty; suffix deletion against independent checkpoints;
key/replica lifecycle; insufficient-space behavior; and separation of custody, access and export.
Demonstrate the claimed durability and retrieval behavior on each supported storage backend before
advertising an evidence-preservation service.
