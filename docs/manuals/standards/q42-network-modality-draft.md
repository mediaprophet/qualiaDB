# Q42 Networking Modality

**Status:** Proposed semantic modality and storage profile; no physical ABI change or opcode assignment

**Date:** 2026-09-06

**Working profile identifier:** `urn:qualia:q42:profile:network:1`

## 1. Decision

Add a **networking modality** to the Q42 specification: a versioned vocabulary, evidence schema,
validation/compilation contract and bounded execution view for network facts and governed operations.
It supplies the semantic substrate for the independent
[Qualia Peer Runtime](./qualia-decentralized-network-fabric/peer-runtime.md), using the existing core
instead of creating a separate networking database.

The first profile preserves the 48-byte `NQuin`, its six fields, existing tag ownership and unified
Q42 storage. Network identifier kinds belong in graph relations; they do not consume new object
datatype tags. New evaluator operations, if needed after reuse of current modalities, require a
separate opcode/role review in the canonical FrameLayout registry. No numeric value is allocated
by this draft, and no packet is executed merely because it decodes into a Quin.

## 2. Record capacity and the 60-bit clarification

The [format specification](./q42-format-internal-draft.md#7-canonical-physical-layout) and
[current NQuin definition](../../../crates/qualia-core-db/src/lib.rs) establish this physical layout:

| Byte offset | Field | Bytes | Bits |
|---:|---|---:|---:|
| 0 | subject | 8 | 64 |
| 8 | predicate | 8 | 64 |
| 16 | object | 8 | 64 |
| 24 | context | 8 | 64 |
| 32 | metadata | 8 | 64 |
| 40 | parity | 8 | 64 |
| **Total** | **Five non-parity fields plus parity** | **48** | **384** |

Thus the physical non-parity region is **40 bytes / 320 bits**, including metadata and packed
control/type information. It is not 40 bytes of unconstrained application payload. The current
implementation is not 42 data bytes plus six parity bytes. The older approximate “42 semantics +
6 computational support” wording in ADR 0008/source commentary is not a byte-accurate map.

The **60-bit** quantity describes a particular dictionary/inline object payload: with the pointer
flag clear, bits 60–62 are the inline datatype and bits 0–59 contain its payload; bit 63 has a
separate pointer interpretation. It is neither the capacity of a whole Quin nor a maximum size for
an identifier, contract, signature or Q42 artifact. Larger exact values are represented through
lexicon/artifact references and multiple related Quins. Identifier hashes remain indexes with a
full-value collision check, not cryptographic identity proofs.

Networking therefore does not need to shrink its entire record into 60 bits. A route can have many
Quins for its target, realm, expiry, issuer and evidence. A compiled forwarding entry can be a
caller-owned struct derived from those Quins, subject to its own ABI/size review and the same pass
budget; the persisted semantic datum remains canonical `NQuin`.

## 3. Three representations with explicit authority

| Representation | Stored as | Authority and lifetime |
|---|---|---|
| Exact network evidence | Original signed bytes and typed full digests in core-managed Q42 artifact/record storage | Signature, issuer, scope, epoch, expiry and withdrawal checked; immutable evidence may remain after authority expires |
| Semantic projection | Canonical Quins plus exact lexicon references, profile and source-evidence links | Derived interpretation with pinned schema/ontology; not a substitute for the source signature |
| Compiled network view | Bounded local tables, immutable generations and leased evidence handles | Valid only under the compiled authority/policy/schema generation and deadline; rebuildable, never a portable authorization credential |

The first two use existing Q42 volume/index/generation owners. If the core cannot preserve arbitrary
signed bytes exactly in its current artifact facilities, implement a versioned payload profile there
before claiming this feature. Do not put opaque proofs into model `.p64` or KV-weight pages, or imply
that graph projections alone provide lossless signed-byte storage.

Hot session keys, nonce state and in-flight packet buffers stay in protected ephemeral arenas.
Network measurements and audit receipts may be durable under explicit retention/privacy rules;
serializing an active session into a graph must never persist its traffic secrets.

## 4. Vocabulary and record classes

The working vocabulary namespace is `urn:qualia:q42:network:`. It is a proposal, not a published
ontology. Its pinned bundle must include term definitions, SHACL shapes, supported reasoning rules,
quantity semantics and CBOR-LD compression mappings before interoperability.

| Record class | Required meaning | Typical lifecycle |
|---|---|---|
| `NeighborObservation` | Locally observed bearer/adjacency, observer, method, time and uncertainty | Short-lived hint; cannot establish controller identity |
| `RouteAdvertisement` | Persistent target, DNI/realm, full issuer evidence, epoch/sequence and validity | Signed admission, refresh/withdrawal, retained historical evidence |
| `ProviderGrant` | Who may serve/relay which resource to which audience for what purpose | Scope-bound grant with independent expiry/revocation |
| `ServiceDescriptor` | Service IRI/version, schemas, operations, limits and contract/profile requirements | Signed, versioned, compiled on cold paths |
| `AgreementBinding` | Exact CBOR-LD contract and pinned semantic-bundle digests, ratification and duties | Negotiated/ratified, then current policy and budget state |
| `SubscriptionCheckpoint` | Authorized view, filter/projector contract, applied frontier and retention epoch | Atomic with durable projection progress; invalidated by visibility changes |
| `SyncCheckpoint` | Authorized operation set, full digest/proof profile and causal frontier | Signed evidence; not proof of omitted global state or automatic merge authority |
| `ResourceMeasurement` | Quantity, unit, device/human/airtime scope, interval, attribution and uncertainty | Measured/estimated/unknown joules and seconds; immutable observations with corrections |
| `DeliveryReceipt` | Exact operation/artifact, signer, accepted stage and optional obligation reference | Stored/retrieved/applied/settled remain distinct facts |
| `AuthorityWithdrawal` | Issuer, withdrawn authority, ordered epoch/sequence and effective scope | Invalidates derived handles before further affected delivery/commit |
| `AdmissionPlan` | Local derivation from verified target/service/policy/evidence and resource state | Nonportable compiled view; never obtained by trusting a remote serialized handle |

Every class declares whether it is source evidence, derived assertion or local observation. A valid
signature proves its origin under an accepted key, not its truth. Conflicts, uncertain timing and
unsupported semantics remain explicit non-allow states where authority is needed.

Illustrative graph structure, before profile freeze:

```turtle
@prefix qnet: <urn:qualia:q42:network:> .
@prefix ex: <urn:example:network:> .

ex:route17 a qnet:RouteAdvertisement ;
    qnet:target ex:resourceA ;
    qnet:evidence ex:exactSignedRoute17 ;
    qnet:authorityScope ex:communityScope ;
    qnet:semanticBundle ex:pinnedNetworkBundle ;
    qnet:validity ex:routeValidity17 .

ex:usage17 a qnet:ResourceMeasurement ;
    qnet:energy ex:scopedJouleObservation ;
    qnet:time ex:scopedSecondObservation .
```

The example omits values/proofs deliberately and is not a conforming complete record. Production
shapes require exact targets, issuer authority, typed digests, intervals and quantity availability.
No absent energy or time observation is silently converted to zero.

## 5. Quin encoding and existing modalities

Networking facts initially use ordinary graph/proposition predicates with full lexicon recovery.
Do not interpret the low byte of a plain predicate hash as an executable networking opcode.
Compilation selects the evaluator role explicitly. Multiple Quins can describe one signed source
object; each carries the correct graph/sensitivity/routing role and source linkage.

| Field | Networking profile rule |
|---|---|
| subject | Local reference to a network assertion/entity, collision-checked against full identity |
| predicate | Ordinary property reference for stored facts; a reviewed compiler may produce an existing modality instruction in its distinct role |
| object | Canonical existing inline datatype or lexicon/artifact reference; no new tag for “peer”, “route”, DID or digest algorithm |
| context | Authorized graph/context under its established sensitivity interpretation; never a raw session pointer |
| metadata | Only the selected canonical role's fields; all other roles' overlapping fields are unavailable |
| parity | Current five-field fold, recomputed after any field mutation; never repurposed as network payload |

Reuse deontic permissions/duties, temporal expiry/trace reasoning, epistemic evidence confidence,
paraconsistent conflict isolation and supported resource logic where their actual semantics fit.
Network facts can be inputs to those modalities without a new network opcode. Unimplemented,
truncating or ambiguous compilers cannot satisfy a normative validation claim.

[FrameLayout](../../../crates/qualia-core-db/src/foundation/frame_layout.rs) explicitly treats metadata
as role-exclusive overlays. Quin type bits 60–63 overlap routing lane bits 61–62; other roles overlap
sensitivity, tensor clocks and flags. Setting a new networking type nibble on a routed Quin can
change its lane. The first profile identifies networking kinds in graph relations and compiled
table types, leaving that overlay untouched. Any later optimized packing needs a complete role
matrix, collision tests and old/new reader behavior, not a claim of spare universal bits.

## 6. Compiled admission plans

A cold compiler takes verified source evidence, target/service identity, selected ontology/contract,
current authority/policy generations and an operation's resource requirements. It emits a bounded
plan containing:

- collision-checked target/service and evidence handles;
- permitted action, context, purpose, audience and sensitivity;
- validity deadline and invalidation dependencies;
- supported message/filter/merge/crypto profile handles;
- candidate route/provider class and disclosure limits; and
- the resource reservation policy and receipt requirements.

Its key commits to the full input references and compiler/profile versions using the selected
cryptographic digest, not just a `q_hash`. Reuse a plan only when all scope/generation/deadline
conditions still hold. Actual mutable budget balances remain current counters; caching a plan
never caches permission to spend an old balance again. New observations do not rewrite signed facts.

Fast execution uses the plan to constrain lookup, forwarding/service selection and application
delivery. No ontology resolution, arbitrary graph expansion, whole-volume loading or asymmetric
signature verification is introduced into per-packet forwarding. A miss or invalidated dependency
returns bounded pending/deny/recompile behavior. Source receipts and derived plans share Q42 evidence,
so authorization is not independently reconstructed differently by every service.

This mechanism is one of QPR's intended innovations: compile semantic authority into reusable
network execution views while retaining auditable source meaning and explicit invalidation.
Performance and safety gains require implementation evidence.

## 7. PQ proofs and Q42 integrity are different layers

[QPR's PQ profile](./qualia-decentralized-network-fabric/post-quantum-security.md) carries larger keys,
dual signatures and typed SHA-384 commitments outside individual Quin payloads. Q42 v3's existing
32-byte header root remains its own field/algorithm. Do not write a 48-byte digest into that slot or
silently relabel SHA-256 as SHA-384. Store a signed PQ manifest with full commitments over the exact
required artifacts/sections through a versioned evidence profile. Only if a future physical header
revision is approved may its root layout change.

The current Quin parity operation is an XOR checksum, not a cryptographic authenticator, a proof of
authorization, or a specified Reed–Solomon correction code. Attackers can recompute it. Signatures,
AEAD and full content commitments provide the relevant security boundaries independently.

There is a concrete implementation mismatch to resolve before networking integration:
`NQuin::calculate_parity` / `verify_ecc_parity` fold all five non-parity fields, while the inspected
`frame_layout::parity` / `sealed` / `parity_valid` helpers still fold four and omit metadata. For
nonzero metadata these can disagree. Networking producers must use the canonical five-field
operation and cannot accept either checksum opportunistically. P15 tracks a caller audit and
fixture/migration classification; this documentation change does not silently rewrite stored data.

## 8. If a true 42 + 6 layout is desired

That would mean **336 payload bits plus 48 parity bits**, still totaling 384 bits. It is a different
physical record encoding, not a networking modality or an interpretation of the existing fields.
A decision would need to specify the six bytes of redundancy and its error model, where the extra
16 payload bits live, all offsets/overlays, and CPU/GPU/WASM alignment and decoding behavior.

Evaluate a versioned storage encoding with conversion to the existing runtime ABI separately from
a universal ABI replacement. Either needs exact vectors, proof of capacity/bounds, old-reader
rejection, mixed-version negotiation, semantic/parity migration and affected caller audits. A
40-to-42-byte raw non-parity increase is 5%; it does not by itself justify those costs or make
identifiers/signatures fit inline. This proposal preserves current writes and does not allocate a
new container version or redefine the immutable 48-byte NQuin contract.

## 9. Implementation and acceptance

QDNF-P15 in [the implementation programme](./qualia-decentralized-network-fabric/implementation-conformance.md#qdnf-p15--q42-networking-modality)
owns this profile alongside core/Q42/FrameLayout maintainers. Required evidence:

1. Exact 48-byte layout, five-field parity, role/collision tests and nonzero-metadata round trips on
   native/WASM/GPU-facing representations; explicit handling of incompatible old fixtures.
2. Pinned ontology/SHACL/CBOR-LD vectors, lossless source storage, full-value hash-collision checks,
   large digest/proof references, and rejection of unknown executable/critical profiles.
3. Scoped lookup/subscription/projection, verified conflict handling, current generation/revocation
   checks, aggregate reservations, and explicit units/unknown resource observations.
4. Bounded compile/query/rebuild, atomic evidence/index publication, crash recovery and no key leakage.
5. PQ manifest verification independent of legacy container checksum/root, including substitution,
   tampering, truncated digest and unauthorized projection cases.

Generic Q42 readers may store/inspect networking graphs without supporting QPR execution. They must
not claim validated network authority or act on unsupported critical profile semantics. A semantic
profile addition alone does not require changing the physical Q42 volume version.
