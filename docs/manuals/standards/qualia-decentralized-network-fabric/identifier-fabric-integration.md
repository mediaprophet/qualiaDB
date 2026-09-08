# Identifier Fabric Integration with QDNF

**Status:** Proposed networking integration; consultation terminology, not a frozen schema or implementation

**Date:** 2026-09-06

**Source baseline:** Identifier Fabric consultation at commit `6601dc811468d266f4ac952243ce65fb10372606`

## 1. Integration decision

QDNF uses a fabric of typed instruments and relations to determine what is requested, how it can
be reached, which authority applies, and what evidence supports an outcome. **Identity is not an
identifier.** A DID is a decentralized identifier; its controller proof does not identify a living
person in full. The [consultation brief][brief] and [architecture spine][spine] supply this distinction.
This document develops the authorized networking design; it does not declare the consultation,
ontology, Host bindings, or runtime implementation complete.

| Plane | Networking interpretation | Required separation |
|---|---|---|
| Entity/agent typing | Distinguish `NaturalAgent`, AI-agent, organization/service, machine/device, and living non-human referents | A living human remains separately enumerable and unmerged; software, office, legal personality, or device does not become that person |
| Claim/opinion | Assertions, grants, assessments, observations, and their epistemic/deontic context | Signature validity, assertion truth, permission, and culpability are separate questions |
| Spatiotemporal handle | Realm/cell reachability, DNI, observed path, environment and validity interval | Where/how now does not establish personhood or continuing authority |
| Instrument | DID, key, credential envelope, locator, account, wallet, proof, or content reference anchoring a typed relation | Possession, naming, payment, or successful authentication does not merge the related parties |

Keep `NaturalAgent` in the consultation's living, SHACL-first representation. Guardianship does not
merge guardian and ward; an AI-agent's operator remains a separate relationship. Device user,
account holder, telecom subscriber, and device are independently evidenced relations. Pseudonyms
are scoped alias instruments: creating one neither creates a second person nor authorizes joining
existing person records. The design selects no universal natural-agent join key.

## 2. Proposed vocabulary bindings

The [taxonomy][taxonomy] and [SHACL split][shapes] names below are consultation bindings. `idf:` is
the proposed namespace in that source, not a published QDNF registry or an installed shape pack.
Shape names identify validation proposals, not interchangeable RDF instance classes. Pin exact
ontology, shapes, rule, lexical-context and codec artifacts through [Ontological Contracts](./ontological-contracts.md)
before a future executable profile uses them; unsupported constraints yield a non-allow result.

| Consultation term or shape | Proposed QDNF binding | Non-entailment |
|---|---|---|
| `NaturalAgentShape`, `AiAgentShape`, `MachineDeviceShape`, `OrganizationShape` | Explicit participant type behind a scoped reference | One shared identifier/type cannot substitute for all participants |
| `RelationScopedLocatorShape`, `PseudonymAliasShape`, `RelationLifecycleShape` | Directed alias/locator assertion with purpose, audience and temporal lifecycle | Address equality does not merge people or counterpart relationships |
| `DniShape`, `RarShape`, `QSessionProofShape` | Separate reachability, route-publication and session-proof records | Session authentication does not confer route-update or controller-signing authority |
| `SymbolicPermissionContextShape`, `RoleCapacityGrantShape`, `GroupAuthMembershipShape` | Provider mandate, current role/membership and accepted service commitment | Role occupancy or group membership alone does not authorize arbitrary operations |
| `PurposeBindShape`, `AntiCoercionConstraintShape`, `DelegationChainShape` | Purpose, target, conflict and delegation constraints checked by QPolicy | A proxy, new provider, or extra signature cannot remove an upstream restriction |
| `CoAttestationBundleShape` | Scoped evidence evaluated against an explicit authorization recipe | Member count and role diversity alone establish neither independent authority nor identity |
| `AccountabilityArtifactShape`, `AssertionClaimShape`, `RelianceRecordShape` | Attributed receipts, claims, reliance and outcome evidence | Grant success does not discharge accountability; an attributed signature does not establish culpability |
| `AssumptionChainShape`, `RootCauseBundleShape`, `BlastRadiusMapShape` | Time-indexed review of invalidated assumptions and their consequences | Retraction or adverse findings do not rewrite person types or merge affected parties |

Network-provider roles and cell assignments follow [Semantic Network Roles](./semantic-network-roles.md).
That model's mandate, offer, agreement, commitment, resource envelope and execution assignment
remain separate records. Its networking modality can reference these fabric bindings; this document
allocates no opcode, wire field, Host method, or additional runtime identity service.

## 3. Resolution and authority are different operations

[Identifier and Resolution](./identifier-resolution.md) continues to define each identifier's job.
A full DID/resource reference names the intended controlled subject; method-specific verification
relationships establish the authority relevant to that operation. A signed RAR establishes an
authorized reachability assertion, DNI supplies current topology, and QSession proves its selected
endpoint and request context. A bearer observation proves none of those authorities by itself.

Q42 resource coordinates, observer references and `q_hash` values remain local/indexing tools.
Content digests establish exact-byte identity, not human identity or access permission. Dereference
compact indexes and verify full identifiers, digests and applicable proofs at security boundaries.
Controller signing, route update, session authentication, transport protection, capability
presentation and discovery remain purpose-separated, even when instruments share a controller.

Relation locators bind direction, parties/context, audience, permitted use, issuer/controller,
validity, revision and withdrawal. Two directed mailboxes are two instruments. DNS, email, Solid,
WebID and similar legacy forms may provide explicit compatibility paths; their location or login
success supplies no new native authority. Scope resolution caches and negative results to the
requester, relation, purpose and policy epoch; expire or invalidate them when the binding changes.
Private alias-to-person relations and full counterpart lists are not public discovery records.

An inferred `SAME AS` association MUST NOT execute as person-record merge, authority substitution,
or alias correlation across privacy scopes. If retained, represent the candidate as a non-entailing
claim with provenance, epistemic state and audience; do not install an active `owl:sameAs` edge that
allows a general reasoner to perform the forbidden merge. Even a confirmed instrument association
requires separately authorized disclosure and never widens a grant.

## 4. Operation lifecycle across roles and cells

1. **Describe intent:** bind operation identity, requesting agent/authorized principal relation,
   target resource, service/version, purpose, permitted outputs/side effects, scope and deadline.
   Record the role mandate, grant/delegation chain, agreement and interpretation-bundle digests.
   A local cell/lease handle supplies execution ownership; it is not a transferable capability.
2. **Resolve candidates:** use only the relation locators and disclosures permitted for that
   intent. A classifier or lexical alias may suggest a candidate; it cannot grant, equate people,
   select an unknown key role, or promote an observed location into controller authority.
3. **Validate temporal bindings:** verify issuer/controller relationships, current role occupancy,
   membership, revocation, proof purpose, locator/RAR validity and any environment predicates.
   Record effective time, observation time and clock uncertainty separately. Missing/stale evidence
   gives a bounded pending or non-allow outcome, not a new person or guessed authority.
4. **Admit execution:** atomically reserve resources under the accepted commitment and install a
   bounded policy decision bound to the authorized agent, role, cell assignment and generations.
   Expose only the assignment evidence a peer needs; local handles and private relation graphs
   stay local. Joules, typed seconds/compute, bytes and accepted funding remain separate dimensions.
5. **Deliver and commit:** recheck current authorization at each delivery/commit boundary as in
   [the runtime API](./peer-runtime-api.md). Delegates and replacement cells inherit purpose,
   target and output restrictions. Election, reassignment, withdrawal or changed environment
   requires revalidation; transport rekey or path migration does not refresh a role grant.
6. **Finish or recover:** preserve operation identity, accepted bindings and stage-specific
   receipts. Cancellation stops new effects and drains pinned leases; ambiguous external work
   reconciles under the same identity. Resume requires current authority and the committed
   application/checkpoint state specified by [Semantic Peer Services](./semantic-peer-services.md).

Purpose binding is enforced before provider selection, delegation and disclosure, not merely
written into a later log. A prohibited target/purpose cannot become allowed by asking another
agent, moving cells, presenting a badge, paying, or collecting co-signatures. Where purpose or
conflict facts remain unverified, do not treat a declared purpose string as proof. Sanctuary/Webizen
owns the consent and anti-coercion interaction; QDNF carries the scoped decisions and enforcement
requirements without inventing a new UI or granting access from diagnostic suggestions.

## 5. Co-attestation without authority from counting

Bind every counted attestation to the same operation/claim digest, purpose, target, audience,
context, membership/role epoch and bounded validity window, with replay protection. Validate the
signer, verification relationship, exact signed bytes and authority to attest that predicate.
Select a recipe authorized for the requested action before evaluating its members; a bundle
cannot select its own convenient quorum, trust roots, roles, or independence assumptions.

The recipe states required predicates, qualifying authority domains, permitted thresholds,
dependencies and evidence of independence. Deduplicate instruments and common control/delegation
roots according to that recipe. Two keys, two labels, two cells, or two signatures from one source
are not automatically two independent authorities. If required independence cannot be established,
report that evidence gap. SHACL minimum counts and diverse `keyRole` values are structural checks,
not authorization proofs. Threshold recovery of key material likewise does not authorize its use.

Evaluate actual proof artifacts under the selected verifier/profile; a boolean “trait proven” or
successful secrecy check cannot grant a capability. A proof establishes its bound predicate under
its stated assumptions, not the whole person, real-world truth, or permission for an unrelated act.
Neither co-attestation nor lexical/ML confidence overrides a denial or enlarges a delegation.

## 6. Evidence, storage and recovery

Use exact core artifacts, including candidate QNF, and typed Q42 projections as described in
[Core Storage and Cache](./core-storage-and-cache.md). Keep actor, instrument, role, cell assignment,
statement, verification result and temporal binding separately attributable. Indexing or compaction
must preserve their distinctions and required source bytes. Private keys, passwords, raw biometric
samples and unrestricted relationship graphs are not routine packet evidence or diagnostic data.

Access authority and [Electronic Evidence and Retention](./electronic-evidence-and-retention.md)
authority are separate. A receipt may establish what an instrument signed or what a service
observed; it does not by itself establish who operated it, knowledge, intent, responsibility, or
culpability. Record these as separately supported claims with uncertainty and review provenance.
Correct false assertions with supersession/retraction and scoped reliance/impact analysis; preserve
required history under the retention policy rather than silently rewriting it or broadcasting it.

Key loss leaves control recovery **pending** until the applicable recovery policy verifies surviving
authority and authorizes replacement bindings. It neither erases a person nor mints another one.
Keep old instrument/role bindings and their intervals so historical operations remain interpretable.
Compromise introduces uncertainty about affected assertions and windows; do not automatically
reattribute all past acts, clear all prior evidence, or claim every old signature remains trustworthy.

Replacement keys, controller grants and role occupancy require explicit temporal transitions.
Recovery does not automatically upgrade old evidence to post-quantum assurance, approve a new
algorithm, or bypass [the post-quantum profile](./post-quantum-security.md). Preserve original suites,
verification assumptions and compromise status; a fresh countersignature attests its actual claim
and time, not retroactive protection of an earlier signature. Live discovery and access still
revalidate independently of historical evidence retention.

## 7. Consultation decisions and open items

| Source / section | Claim or gap | Risk | Networking proposal / status |
|---|---|---|---|
| [Spine][spine] §§3b–3d; [F2][shapes] §5 | Participants, claims, handles and instruments must remain distinguishable | DID/device/guardian substituted for a person | Adopt four-plane records and explicit non-entailments; proposed integration requirement |
| [Spine][spine] §§3g, 3j–3n; [F1][taxonomy] §26 | Locators and role/delegation bindings evolve in time | Stale office authority or proxy-laundered purpose | Bind operation, role and cell assignment; revalidate changes and inherit restrictions |
| [F2][shapes] §5.5 | Member count/diversity is insufficient to establish independent authority | Repeated common-control signatures satisfy a nominal quorum | Add action-specific authority/dependency recipe; its concrete format remains open |
| [F6][classifier] §§2–4, 13–14; [F2][shapes] §26 | F6 retains older “no dedicated shape” notes while F2 now sketches those shapes | Treating consultation names as implemented or frozen | Bind exact artifact versions; reconcile lane notes before executable schema adoption |
| [Brief][brief] §2c; [F6][classifier] §§3–4 | SAME AS and lexical suggestions cannot identify or authorize a person | Cross-plane merge or hidden cross-context correlation | Keep suggestions non-entailing; symbolic validation precedes any authorized binding |
| [Spine][spine] §3o; [F2][shapes] §27 | Attribution, reliance and culpability are distinct claims | An AI-agent's false claim becomes an automatic accusation against a human | Preserve source, role/time and uncertainty; separately authorize evidence access and review |
| [Brief][brief] §6.1; [F1][taxonomy] §6 | Natural-agent join-key policy remains open | Universal correlator chosen by networking convenience | No universal join key selected; review contextual association, recovery and privacy requirements |
| [Brief][brief] §§2b, 6; [crosswalk][crosswalk] | Primary diagram intake and broader consultation decisions remain outstanding here | Claiming consultation-complete or assigning unapproved implementation scope | Retain review provenance; P21 owns the integration follow-up, not automatic runtime promotion |

Track this design under [QDNF-P21 — Identifier Fabric integration](./implementation-conformance.md#qdnf-p21--identifier-fabric-integration).
Acceptance scenarios include expired role turnover, proxy purpose mismatch, correlated co-signers,
forged proof-success flags, alias/SAME AS merges, classifier suggestions, cell reassignment,
recovery with disputed keys, historical attribution and cross-scope evidence disclosure. These are
required future fixtures, not tests run or implementation guarantees established by this document.

## 8. Source review and implementation boundary

Read at the pinned commit: [brief][brief] in full; [spine][spine] §§1–7; [taxonomy][taxonomy]
§§1–9 and 26–27; selected [SHACL][shapes] tables in §§3–5.5, 7–9, 19, 21–22 and 25–27;
selected [diagnose][diagnose] tables in §§2–5, 9 and 11–14; [classifier][classifier] §0 and selected
§§2–4, 13–15; and the [attachments crosswalk][crosswalk] in full. Primary diagram images were not
visually reviewed: attempted Diagram2, Diagram4 and the 20230119 PDS/PCT image returned fetch errors.
Diagram interpretations here are crosswalk evidence only; no complete diagram review is claimed.

The checkout's [identity_fabric.rs](../../../../crates/qualia-core-db/src/modalities/identity_fabric.rs)
was read separately. `identity_survives_loss` counts remaining anchors;
`enumerated_identity_confidence` divides counts; `web_of_trust_confidence` applies scalar decay;
`zkp_capability_granted` accepts two booleans without verifying a proof or grant.
`recompute_fabric` filters compact IDs and stops when output fills. These are implemented local
helpers, not the classifier, an independent-authority verifier, complete recovery, or this QDNF
admission lifecycle. Their names and passing local examples cannot support those stronger claims.
The F6 classifier document is itself a proposed inference constraint map, not shipped enforcement.

[brief]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/IDENTIFIER_FABRIC_CONSULTATION_BRIEF.md
[spine]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md
[taxonomy]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/CRYPTO_INSTRUMENT_TAXONOMY_WIP.md
[shapes]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md
[diagnose]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md
[classifier]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/alice-f6-classifier-symbolic-binding-pressure-test.md
[crosswalk]: https://github.com/mediaprophet/qualiaDB/blob/6601dc811468d266f4ac952243ce65fb10372606/docs/work-in-progress/IDENTIFIER_FABRIC_ATTACHMENTS_CROSSWALK.md
