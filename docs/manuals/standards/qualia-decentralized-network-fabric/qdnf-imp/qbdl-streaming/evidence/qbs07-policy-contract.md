# QBS-07 existing-owner policy and closure contract

2026-11-12. **Root-directed candidate: structural phase A and local pinned/admin phase B implemented; full QBS07 not qualified.** Initial inventory was source inspection only. Authorized source additions are recorded below; this worker ran formatting only, not builds/tests, signatures, network fetches, live policy adoption or publication. Existing identity, governance, crypto, consent and semantic owners remain authoritative.

## Reuse inventory and trust limits

Read governance/identity DIRECTORY_INDEX.md before searching actual sources across crates. Their June inventories predate newer modules. Paths below are relative to crates/qualia-core-db/src except explicitly named client paths.

- **AuthorityOwner** (net/qdnf/authority/owner.rs:111,185,231) owns eight full-digest principal/recipient/purpose/instrument/scope/operation grants. install_grant_with_ceiling explicitly trusts host admission; it does not verify consent. issue_permit checks current revocation/contact/time/full binding/ceiling. Reissue through the SAME live owner at every acceptance. validate_handle checks association only.
- **ExecutionPermit** (authority/permit.rs:122,151) has private fields, but is Copy and not self-revoking. current_at checks expiry only; matches_grant checks time/purpose/audience. MutationHandle contains slot/generation/principal but no owner incarnation: similarly initialized owners can match. Retain the concrete owner borrow/locked guard; never accept a replacement owner on recheck.
- **QPolicy** (authority/policy.rs:194, session/policy.rs) already distinguishes Allow, Deny, Challenge, NeedsHuman, Incomplete and Error. Only current Allow admits; wire enums/booleans are not authenticated decisions.
- **VerifiedLabel** (policy_labels/verify.rs:11,40) establishes exact decoding/digest consistency, NOT issuer signature or recognized issuer authority. Authenticate issuer and exact bytes before joining.
- **JobLabelContext / protected_read / egress** (policy_labels/flow.rs:14,25,52; egress.rs:22,41,55) provide existing monotonic labels, recipient/compartment/purpose/environment and cache checks. Public LabelFields and caller C0 requests are not authenticated labels. Discard an attempted join on error; state can be partially updated. release_derivation (release.rs:10) confines release to the original issuer, but its supplied issuer digest still needs authenticated consent.
- **ConsentGrant / ConsentLedger** (governance/consent_contract.rs:150,346,364,389) provide real Ed25519 signatures and fixed 32-slot replay/revocation state. Prefer ledger-backed checks to caller-omittable revocation slices. Category authorization does not compare requested purpose with purpose_hash; verify checks expiry but not created_at as not-before. The principal key is record-supplied. Bind recognized controller, exact purpose/time and complete-source scope separately; do not relabel classical consent as a PQ chain.
- **NativeAuthority GraphScope** (poet_host/native_authority/graph/grant.rs:87, graph/store.rs:193) is genuinely opaque, owner-incarnation/session/generation bound, with pinned project-anchor signature and locked live read checks. Reuse actual owner/provenance. Graph read permission does not authorize policy issuance or arbitrary QBDL disclosure.
- **Authenticated association guards** (poet_host/native_authority/inference/binding.rs:119,132) already hold the actual owner lock and distinguish authentication continuity from policy Allow. Follow this pattern in a streaming child; do not reuse an inference grant as a streaming grant.
- **Semantic dependency owner** (semantic_instruments/resolve/resolver.rs:12, dependency.rs:90) supplies deterministic local plan_resolution/require_digest and required-miss/revocation checks. It is a flat allocating cold planner over public records, not recursive authenticated closure. Require bounded inputs and exact nonempty pins.
- **LocalRegistry** (semantic_instruments/resolve/registry.rs:15,68) preserves collected/activated distinction, but InstalledRelease.closed is caller-set. It cannot mint authenticated closure. DependencyLock::digest concatenates unframed strings: preserve legacy behavior, but do not use it as a new unambiguous security binding.
- **Native instrument attestation** (semantic_instruments/attestation/native.rs:99) verifies origin with the existing VC crypto owner and explicit issuer key. It does not establish recognized authority, substantive truth, recursive closure or PQ authority.
- **WebizenSentinelGate** (poet_host/vibe_sentinel/policy_gate.rs:141,250,284) performs real bounded deontic evaluation, deny precedence, controller/effects/resources and post-execution generation checks. Full-digest bindings must surround compact identifiers. The post-check does not reevaluate expiry using fresh Unix time. Generation bump saturates at u64::MAX; a new bridge must refuse exhaustion.
- **Sentinel pass owner** (governance/webizen/arena_admit.rs:130,210; arena.rs:243) supplies scope/policy/source-bound lease/reset for exact 42 MiB backing. Reuse it; resource admission is not authentication.

Share already has application ownership: crates/qualia-client-core/src/wellfair/api/guardianship.rs:232,248 implements pending live-share requests and owner decisions, checks sanctuary projection restrictions and journals through LiveShareStore. Retain that consent/UI flow and adapt its authenticated adopted decision. It is not a generic QBDL FULL-profile grant. Searches did not identify a general QBDL ShareGrant by that name; that is a scoped naming result, not absence of sharing/governance.

## Proposed evidence chain and nonforgeable closure

Keep three stages distinct:

1. Structural candidate: QBS06 CandidateIdentity/BodyView, decoded labels, catalog records and graph bytes. Hash/parity alone is not identity or permission.
2. Authenticated closure: exact root and every required artifact have verified origin, recognized issuer authority for their role, exact full-digest dependency edges and current owner provenance. Descriptor semantic/policy/consumer refs match these artifacts. Still not disclosure permission.
3. Current use: actual Share/consent, policy and identity owners approve this descriptor/source, recipient, purpose, operation, environment, resources and consumer action now, under a guard held through acceptance.

The implemented AuthenticatedClosure has private fields and is minted only by concrete paired-pin verification plus the semantic structural owner. BoundClosure subsequently associates descriptor/context/source without a hash cycle. LocalWholeSourceGrant separately joins the actual AuthorityOwner. Full integration still requires parent graph revisions and policy/identity/source generation checks; this candidate does not claim those are present. Poet/native integration remains optional. Evidence getters cannot refresh admission. A public caller-implemented trait returning verified/Allow is insufficient.

Remote FULL artifacts compose with QBS03/04 paired COSE and current recognized pin/chain authority. Initial local admission may explicitly map an authenticated native project/session to full principal and installed pin through trusted bootstrap. Native GraphScope's classical signature is not a PQ controller chain. Graph read/write rights alone cannot adopt policy: pin the policy instrument and issuer role first.

Retain concrete owner references or incarnation-bound handles. Async work reenters that same owner. Cache hits/signatures never install grants. InstalledSessionKeys can only be created by the existing session owner after received peer Finished verification; a constructor argument confirmed=true is not handshake evidence.

## Bounded semantic closure

Preserve **16 unique artifacts, 1 MiB aggregate original encoded artifact bytes, existing 42 MiB Sentinel pass**. Root, dependency manifests, labels and policy/consumer artifacts all count; no uncounted sidecars.

Propose fixed 16-node iterative traversal with unseen/visiting/complete states, caller stack/queue and adjacency masks (maximum 256 distinct directed edges). Reject duplicate edges, cycles/self edges, missing targets, contradictory metadata, zero/ambiguous pins and overflow before expansion. Deduplicate identical full-digest artifacts before charging bytes. Required edges cannot disappear because a local catalog calls them optional; optionality requires authenticated schema semantics. No recursion, automatic URLs, wildcard pins or version substitution.

Preserve existing semantic definitions and bytes. Any new closure framing/domain/schema needs QBS01 and semantic-owner review. Do not mutate old lock hashing or invent crypto; use existing SHA384/COSE for reviewed exact-byte bindings. An adopted bounded manifest may be the initial input; an unsigned caller list cannot prove completeness.

Authenticate labels/dependencies, join through JobLabelContext, and seal only on complete success. Existing limits remain 16 compartments, 16 purposes, 4096 label bytes and 64 join inputs; refuse overflow, never truncate. Strict decode must precede public LabelFields accessors, whose slice counts assume validity. Compact NQuin identifiers remain lookup/evaluation values, with full security digests retained separately; parity is never a signature.

Reset/admit the existing Sentinel pass for exact scope/policy/source generations. Preserve rule/step/collision bounds. The small gate's 64 verdicts are not the entire 42 MiB pass. Simulated tickets, absent controller, incomplete computation and exhausted generations cannot authorize production.

## Complete raw-source disclosure

QBS06 plain delivery commits the complete ORIGINAL raw source. Admission covers header, original index names/metadata, every entry/sidecar, gaps/padding, footer and unreferenced bytes. Selected public IDs and valid leaf proofs do not authorize the rest.

Require authenticated adopted whole-source scope bound to SourceCommit.source_digest, exact length and descriptor/context, with audience/purpose/environment/operation. Unknown gap/metadata classification refuses. Public flags, empty dependency lists and generic graph-read grants cannot establish coverage.

Mixed/restricted source or partial rights refuse raw delivery. A future public projection must be a newly constructed/admitted QBDL with sanitized metadata/gaps and its own source digest/closure, not filtered descriptor rows over the original. Private projections/recipient wraps remain Unsupported until integrated. No DEKs/session keys are minted here.

Candidate sinks stay private staging. Names/hashes, index/page bytes, locators/routes, cache-existence responses and logs also disclose information. Apply existing egress/environment/route checks before public sinks, with no permissive fallback. Bootstrap control traffic needs its own bounded nonprotected profile.

## Point-of-use acceptance

Propose with_current_stream_use inside a concrete QDNF acceptance adapter retaining the existing authority/consent/policy owners:

- Revalidate actual session/key possession, full principal mapping, owner incarnation, content/graph revisions and all policy/identity/source generations.
- Consult real ConsentLedger/adopted Share state; enforce recognized issuer, exact purpose/recipient, not-before/expiry, source/descriptor and operation.
- Reissue through the same AuthorityOwner::issue_permit with exact AuthorityBinding and bounded ceiling.
- Obtain fresh Sentinel Allow at trusted current time; apply current joined labels and existing egress/cache/environment checks.
- Revalidate cancellation/generations after staging and immediately before a short bounded acceptance callback under a documented non-reentrant lock order. No awaits/network/unbounded external callbacks under the guard.
- Authorize typed disclosure OR consumer commit. Consumer commit additionally requires QBS12 verified payload/dependencies/profile and a live resource reservation.

A private non-Copy completion receipt is observational, not a grant for the next use. Cancellation before acceptance releases nothing; later cancellation cannot retract already disclosed bytes. Existing owners retain receipt persistence/durability. The staged-call pipeline is useful sequencing evidence; its in-memory receipt sink is not disk durability.

## Root-approved direction and exact file claims

Streaming core must NOT depend on Poet/native host as a mandatory authority. The primary authenticated path is concrete QDNF AuthorityOwner + existing crypto/network/authority::PinAuthority, existing labels/ConsentLedger/Sentinel and the semantic owner. Native GraphScope/Share UI adapters are optional host integrations later.

Phase A authorized source is ONLY semantic_instruments/resolve/bounded_closure/{mod,types,walk,tests}.rs. Result is ClosureCandidate, not AuthenticatedClosure. It validates the supplied structural graph and SHA384 of actual immutable originals, not issuer role or authenticity of the declared dependency list. Parent resolve/mod.rs routing belongs to root.

Approved phase B source lives under bundle/stream/policy/{mod,types,manifest,authenticated,grant,tests,rejection_tests}.rs. It calls existing owners unchanged except the separately approved retirement child below. Root owns bundle/stream/mod.rs routing. Remaining full integration: actual adopted Share/consent, joined labels and fresh Sentinel; consumer/proof/resource gate; guarded publication effect. Native/Poet and client Share adapters are optional, not core prerequisites.

### Concrete paired-issuer authentication

Trusted local pin installation recognizes an issuer's full principal, FULL key pair, validity, audience and exact purpose/role. Use existing PinConfig purpose and KeyPurpose; the adapter supplies a locally recognized closure-issuance purpose when constructing the expected COSE binding. The wire manifest cannot select an issuer role, import its own trusted pair or turn a publication-signing pin into closure/policy authority. One role must not imply all roles.

The signed closure manifest binds exact artifact hashes/edges, supported schema, source and policy/consumer references. Descriptor/context association occurs only afterward to avoid a hash cycle. The frozen manifest format below includes its domain/version in exact canonical bytes; SHA384 of those bytes is the manifest reference. The existing COSE common binding digest slot carries this reference under the fixed closure-specific purpose. Publication wire bytes remain unchanged.

PinAuthority::verify_source(LocalPin, expected_binding, ...) provides the actual paired signature/current recognized-pin check. Its AuthorityReceipt is already private-constructor and owner-bound. Retain its private receipt and recheck through its incarnation-matched PinAuthority; a valid manifest receipt is evidence of recognized issuer authority, NOT a disclosure grant. Independently use the SAME AuthorityOwner::issue_permit for current exact principal/recipient/purpose/instrument/scope/operation, current consent and fresh Sentinel/egress policy. AuthorityOwner's trusted local install remains explicit; no wire bool-to-Allow path is added.

Existing PinAuthority currently supports LocalPin and refuses signed-chain/classical-only input. That permits an honest locally pinned first integration; remote chain acceptance stays Unsupported. Existing local role recognition can use exact purpose-scoped pins without a new crypto primitive. If a signed manifest has several different issuer roles, validate each required role separately; do not promote a single issuer's signature to permission over another issuer's restrictions.

The signed manifest itself and all original artifacts must fit the total 1 MiB closure input budget. Phase A counts artifact originals only; phase B must reserve/count the manifest bytes additionally. Treat the manifest as detached metadata rather than a node hashing itself, avoiding a self-reference cycle. Schema node limits and duplicate semantics require explicit review. No payload disclosure or consumer activation follows merely from phase A success.

## First required fixtures

Positive actual owner/session/issuer, exact closure and whole-source scope; forged parity graph; self-signed unknown issuer; label digest without issuer authority; caller closed=true; missing/cyclic/duplicate/17th artifact; 1 MiB+1 bytes; mismatched semantic/policy/consumer refs; short-ID collision with distinct full digest; wrong owner/session/purpose/audience/source; future/expired/revoked consent; absent controller; stale revisions/generations; generation exhaustion; cancellation during staging; simulated ticket; public entries with private index/gap; forbidden environment/route; successful acceptance then revocation denying next use.

Independently encode a small complete closure and mixed-rights raw source. Every denial yields zero external sink bytes and no consumer commit; private staging may exist. Test actual owners, not fake authorization booleans. Measure cold metadata, bounded closure scratch, Sentinel backing and host storage separately. No executed result is claimed.

## Inspected-source fingerprints

Working-tree SHA-256 at proposal handoff (not execution evidence):

- `net/qdnf/authority/owner.rs`: `b191a9f316d6903956be3392b37cf56bfcffe0252b42378c4a30aade58137527`
- `net/qdnf/policy_labels/verify.rs`: `9de569bec90967916aafead6d4461fb8a6abaf1deaa25a89f4f3d1e072ab16e3`
- `governance/consent_contract.rs`: `548ebe111739d42a67078e53fdfcf1f32c9b4c7d611414ad6bfa6e95a40bd724`
- `semantic_instruments/resolve/registry.rs`: `002db587aedf44696eb6fc3ccc83598d50b624628e4f6cc6e6893a331b62d10c`
- `poet_host/native_authority/graph/grant.rs`: `a0f376f0178d908dd761c83386f025f733a5d021fb02bc9c63412f04c6558d60`
- `poet_host/vibe_sentinel/policy_gate.rs`: `91b0e5e58aba18cf058987e7c519624464db329debbc1578c6246f6578de393d`

## Phase A structural candidate handoff

Implemented fixed limits of 16 nodes, 256 supplied edges and 1 MiB aggregate originals. ClosureNode borrows exact bytes and uses existing StrongDigest and DependencyKind; no new textual semantic ID/version matching or lock/catalog serialization is introduced. Duplicate full-digest nodes (including identical repeats) are rejected; manifest adapters provide one row per digest. Every endpoint/root must exist, all nodes must be root-reachable, cycles/self edges and duplicate edges refuse. A fixed-bitset iterative walk and deterministic dependency-first ordering use no recursion or owned collections. Exact SHA384 originals are checked with the existing crypto owner after cheap graph/size checks. Empty artifact bytes can be structurally hashed; the later semantic schema decides whether an empty artifact is meaningful. Kinds remain unauthenticated candidate metadata.

Five authored regressions cover a diamond with shared dependency and input permutation, missing/duplicate/disconnected/cyclic endpoints, exact hashes and zero digests, 16/17 nodes and 257 edges, and exact/cumulative 1 MiB limits. The largest valid simple 16-node DAG has 120 edges; 256 is the supplied-edge preflight bound, not a claim that 256 acyclic unique edges can exist. rustfmt passed; no builds/tests executed by this worker.

Phase A source SHA-256 at handoff (source only):

- `semantic_instruments/resolve/bounded_closure/mod.rs`: `2cf1db1795a1dc5e45a16ee587c9f82c025b5b6b368bb04a9dabee0c2c7cdaef`
- `semantic_instruments/resolve/bounded_closure/types.rs`: `a9282e77e58b2eff4edbe2a9b766c796e982a1572537f5c09ffaf2b50273c295`
- `semantic_instruments/resolve/bounded_closure/walk.rs`: `1b314f18d36fae11ba3bd6ea5153e8c0b051d2010c12db8315c213cb4d0f90f9`
- `semantic_instruments/resolve/bounded_closure/tests.rs`: `b8abde65c888b6f65afd29cd31ddf22f1ba985c2bc318dc1c296ecd913cc808d`

## Phase B frozen local pinned profile (implementation scope)

The dependency is acyclic: exact QBDL source and artifact originals -> canonical closure manifest -> context semantic_contract -> descriptor. The manifest MUST NOT contain descriptor/context IDs. A descriptor links the complete manifest digest, source digest/length, policy and consumer references; it cannot replace one independently. Source SHA384 alone does NOT authenticate the descriptor entry-tree roots. BoundClosure therefore additionally requires existing PinAuthority paired verification over the exact descriptor ID under fixed purpose qbdl-stream/publisher/v1, using a separately recognized publisher-role pin. Both private issuer and publisher receipts must belong to the same PinAuthority incarnation and pass current recheck at local adoption/use.

Canonical manifest v1 is an exact fixed binary encoding, distinct from COSE and existing lock/catalog bytes: ASCII QBDL-CLOSURE-V1 followed by NUL; source digest[48], source length u64 big-endian, semantic root[48], policy[48], consumer[48], issuer principal[48], role u8=1 (closure declaration issuer), node count u8 (1..16), edge count u16 big-endian (0..256); then sorted unique artifact rows (digest[48], existing DependencyKind tag u8: Ontology1/Lexicon2/Dataset3/LawPackage4/Instrument5/NativeModule6, original length u64 big-endian); then sorted unique edge pairs (dependent sorted-row index u8, dependency sorted-row index u8). Index edges bind full digests through unique rows. No padding/trailing bytes/extension/alternate integer encodings. Max manifest 2048 bytes; manifest+all artifact originals <=1MiB. Zero references, unknown roles/kinds, unsorted/duplicate rows/edges, missing endpoints and self edges refuse. Complete graph/hash validation still uses Phase A; the manifest must match every candidate row/edge exactly.

Manifest reference = SHA384(exact canonical manifest bytes), including magic/version. Paired COSE common binding digest slot carries that reference; fixed purpose is ASCII qbdl-stream/closure-issuer/v1, audience/key IDs/epoch remain exact locally expected values. Existing PinAuthority::verify_source verifies the pair against an installed full purpose-scoped pin; returned principal must equal signed issuer. An arbitrary wire purpose or key never installs authority. AuthenticatedClosure retains the private incarnation-bound AuthorityReceipt and immutable candidate originals, without retaining a PinAuthority borrow. This permits revocation/rotation between staging and acceptance. Recheck takes a concrete PinAuthority whose existing receipt-instance validation rejects a foreign owner. Only the short synchronous current-use call borrows it. It authenticates declared closure only; it does not certify semantic truth or issue disclosure rights.

Local whole-source adoption is a separately named trusted host-admin entry into existing AuthorityOwner::install_grant_with_ceiling. It takes a closure bound to an independently publisher-authenticated descriptor and exact principal/recipient/purpose/operation. The grant instrument is the full manifest reference; scope hashes a fixed frame containing descriptor/context/source digest/length/policy/consumer and whole-source coverage discriminator. It covers ALL source bytes including header/index/padding/gaps/entries; no partial/mixed/unknown coverage constructor exists. The host must establish whole-source rights before adoption, including existing labels/consent/Sentinel where required. This adapter does NOT perform those parent checks and does not claim automatic Share/remote-controller admission.

The adopted wrapper retains the same mutable AuthorityOwner borrow and private handle; no arbitrary owner may be supplied to recheck. It offers cancel/revoke and a one-shot bounded synchronous local-use callback, issuing a fresh permit from that owner and rechecking BOTH closure-issuer and descriptor-publisher receipts through the receipt-affine concrete PinAuthority at host-sampled current time immediately before callback. The pin owner is borrowed only during these short synchronous calls, so staging does not prevent revocation/rotation. No authority receipt escapes the callback. Scope checks bind the exact descriptor and closure. The callback is the trusted host effect boundary, must not await, retain authority, or block indefinitely, and is not itself proof/payload validation, durable commit, resource reservation or consent. Mutation is excluded by concrete Rust borrows during acceptance. Repeated operations require separately adopted grants/operation IDs. Pending cancellation is terminal; failed or panicking acceptance stays spent. Future expiry while trusted callback runs cannot retract already disclosed bytes.

Source claims for this phase are NEW bundle/stream/policy/{mod,types,manifest,authenticated,grant,tests}.rs (split tests if needed); no parent routing edits. Existing cryptographic/authority owners are called unchanged. No new key store, grant registry, crypto algorithm or generic Allow trait. Full QBS07 still requires real upstream consent/labels/Sentinel and consumer integration; this pinned/admin profile is explicitly bounded intermediate evidence.

Root refinement: policy and consumer references MUST occur among manifest artifact rows and therefore be reachable from the semantic root. Detached signature envelope/key workspace use the existing bounded crypto budget separately; manifest metadata remains in the 1MiB closure total.

Root authorized a narrow existing-owner retirement child: net/qdnf/authority/owner/{retirement,retirement_tests}.rs plus only mod retirement in owner.rs. retire_revoked requires the exact live handle and an already-revoked record, clears occupied storage, and preserves global monotonic next_generation. It is not asynchronous/session drain. Old copied permits are historical snapshots; current owner issuance remains required. LocalWholeSourceGrant close/Drop invokes revoke then retire only after its synchronous borrow/effect has ended. There is no automatic eviction; generation exhaustion refuses even with vacant slots.

Phase B tests use a real one-entry BundleWriter/PlainPublisher candidate, actual ML-DSA65+Ed25519 paired signatures and concrete PinAuthority/AuthorityOwner. Negative coverage includes wrong role/key/epoch/audience/owner, canonical metadata corruption/limits, descriptor page-commitment substitution with recomputed candidate ID, revocation/rotation between staging and use, publisher revocation, terminal cancellation/unwind, and explicit parent slot reclamation. These are authored tests, not executed evidence.

## Phase B source handoff fingerprints

Formatting/check passed; no builds/tests run by this worker. Root owns execution qualification. Current SHA256 (not execution evidence):

- `bundle/stream/policy/authenticated.rs`: `89c00cc0f5866dcc96a1c6e4289d8524fbdae44058be216581fe8c2c2d1124e2`
- `bundle/stream/policy/grant.rs`: `69422a22913e7fd03bcc170f0872c1229e94923ea9c206e828643db828bcc1eb`
- `bundle/stream/policy/manifest.rs`: `5c0674328c0cb8d7089dbf17145c4a82f036dc3b85fb8762c05fa49d13a38b63`
- `bundle/stream/policy/mod.rs`: `ad36cd840ff2b3eb63f3ce79bf66d7a1ffbcfe6743c0f7d545167c07827e90d5`
- `bundle/stream/policy/rejection_tests.rs`: `a722fe422f6bed0f28f293f72640a660aa845a4bfd50d888383640fb9aafc336`
- `bundle/stream/policy/tests.rs`: `9adb537ccd099545174e40c160fa0d7079b2e7b00eac063def2536fde3a9e003`
- `bundle/stream/policy/types.rs`: `e16be169b5652c4089e056448eabb946049ce9af2473099a2ccefa7b32b984ab`
- `net/qdnf/authority/owner/retirement.rs`: `14982ddb7abad7ecf8803e12f904cd6c7d5b7436e1048437a69b2ff44522e3ab`
- `net/qdnf/authority/owner/retirement_tests.rs`: `b875254c1a62bb0d683ef1b46b05710e38d806f3473ac66e9789f0adfcecc273`
- `net/qdnf/authority/owner.rs`: `0c43fcca9ff785a189ffd3b8ff9474e7e1148c8cbdb1e529aceb8b6792dee0e6`
