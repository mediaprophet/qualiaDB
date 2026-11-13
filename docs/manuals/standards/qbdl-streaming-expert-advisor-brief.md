# QBDL streaming as a QDNF service extension

Implementation follow-through: [QBDL streaming programme](qualia-decentralized-network-fabric/qdnf-imp/qbdl-streaming/README.md), including HTTP/WebSockets, WebRTC and capability/cost-aware swarm assignments. The brief below remains advisory.

**Expert advisory brief — initial comprehensive draft, 12 November 2026**  
**Status:** Design considerations and proposed requirements for expert review; not a frozen wire specification, implementation-conformance claim, or protocol identifier allocation.  
**Audience:** QDNF, distributed systems, cryptography, semantic governance, Commons economics, and container/runtime specialists.

## 1. Mandate and architectural position

Develop QBDL streaming as a useful **native extension of the Qualia Decentralized Network Fabric (QDNF)**. HTTP is a backward-compatible delivery binding. The protocol must not make HTTP, DNS, a CDN, a payment provider, or a central service necessary for native operation. Its cryptographic methodology should be post-quantum (PQ), consistent with the existing QPR replacement security profile. QPU support is an ecosystem requirement and an illustrative workload, not the organizing principle of this protocol.

The recommended starting point is a **QBDL profile of QDNF's existing content-swarm service**, composed with Share, QResolve, QPolicy, QSession, QSync, and the optional economics service. It should supply bundle-specific discovery, authenticated range descriptions, dependency-aware selection, partial verification, resumability, and governed consumption. It should reuse connection establishment, transport recovery, authority, and accounting mechanisms already owned by the fabric. Whether a separate service identifier is eventually justified is an advisor decision; none is assigned here. [QSession services][qsession]

A purpose-fit protocol is justified because QBDL carries more than a timeline of audiovisual segments. A useful partial transfer may be an ontology and a geometric proxy, a spatial field brick, model initialization and selected weight regions, or a previously solved mathematical problem. Their dependencies, rights, scientific meanings, and resource costs differ. HLS and DASH provide useful prior art for manifests, adaptation, segment addressing, and availability, but should not dictate the fabric's architecture.

The intended outcome is one semantic and security contract with native QDNF and HTTP bindings. HTTP compatibility must not create a weaker path around native governance. Existing classical clients may consume explicitly published legacy representations, but that path must not be presented as conforming to the new PQ security profile.

### 1.1 Decisions requested from the advisor

1. Confirm the content-swarm extension boundary and its composition with the existing service graph.
2. Select an interoperable PQ trust and range-proof profile, including encrypted storage and HTTP compatibility.
3. Define a bounded descriptor and dependency model that preserves existing bundle bytes where practical.
4. Reconcile Commons publication, finite compensation, private custody, and reusable computation with streaming.
5. Specify release gates, test vectors, and implementation ownership before freezing wire fields.

### 1.2 Evidence and terminology

This brief combines inspected source, repository design documents, and proposed integration work. Source baseline: QualiaDB Git HEAD `afbe431195167fde0fadc233ed79fd2fe85f73f0`, with uncommitted development changes present during review. Links refer to the current tree, not an immutable release. Design documents marked proposed remain proposed even where supporting modules exist. Older documents sometimes overstate guarantees or disagree with newer profiles; those conflicts are identified rather than silently reconciled.

Here, a **bundle** is an immutable QBDL byte object; an **entry** is an intact embedded file; a **range** is a byte interval; a **segment** is a declared independently admissible transfer unit; and a **representation** is one explicitly related alternative for a purpose. A **snapshot** fixes a generation and its dependency commitments. A segment need not represent time. Transfer completion, cryptographic verification, semantic validation, authorization, application, and settlement are separate states.

## 2. Existing foundation and immediate gaps

### 2.1 QBDL is already range-addressable

The core HMC bundle profile uses magic `QBDL`, version 1, a 64-byte little-endian header, intact payload entries, and a CBOR index at the end. The header supplies index offset, index length, and total length. Entry records include key, kind, absolute offset, length, SHA-256 digest, and optional metadata. The whole-file CRC-32C covers the bundle with its CRC field zeroed. Entry-relative offsets remain unchanged inside embedded files. [Bundle format][format]

A client can obtain the header, fetch the footer index, then request selected entries or interior regions. That is random-access progressive retrieval, not evidence that an unfinished QBDL file can be consumed as a live append-only stream. A producer ordinarily needs finalized layout and length to publish this header. Native and HTTP delivery should share the same object model.

Default outer alignment is 64 bytes; an optional writer mode uses 4096 bytes. Neither automatically guarantees an embedded format's larger absolute alignment. A P64 consumer's page assumptions must be checked against the actual bundle offset. Relative offset preservation and zero-copy alignment are distinct claims.

There is also a ZIP-based HMC profile elsewhere in the ecosystem. The `.hmc` suffix alone does not select QBDL. Negotiate and inspect actual format/profile identifiers; do not pass ZIP offsets to a QBDL range planner.

### 2.2 Evidence matrix

| Existing element | What inspection establishes | Streaming integration still needed |
|---|---|---|
| QBDL writer and reader | Intact entry bytes, index, whole-file CRC and entry digest fields | Native service contract, trusted descriptor, partial-object assurance |
| `HmcHeaderProbe` and `HmcProgressiveRangePlanner` | Header/index acquisition and prioritized entry/interior range plans | Bounded remote admission and authenticated planning metadata |
| `verify_arrived_range` | Length check; optional SHA-256 and/or CRC comparison | Mandatory accepted proof mode; no unchecked range labeled verified |
| `BundleReadLimits` | Separate bounded index preflight and full-buffer validation | Equivalent streaming invariants without retaining the whole object |
| Governed hydration workers | Capability masks, fuel/byte limits, tiered work | Authenticated remote grant compilation and aggregate reservations |
| Entity manifest and variant matrix | Geometry references, affordances, variants, Commons obligation fields | Verified authority, precise encoding, dependencies, current economic state |
| QDNF/QPR documents and source | Service, session, consent, connection-fabric and runtime concepts | Specific bundle profile and end-to-end conformance evidence |

Sources: [range loader][loader], [bounded admission][limits], [hydration][hydration], [worker pool][workers], [entity manifest][manifest], [variant matrix][variants], [QPR API][runtime].

The following findings deserve explicit release gates:

- The progressive index path decodes with `ciborium` and checks entry bounds, but does not inherit all strict preflight checks. Count agreement, duplicate keys, overlapping entries, digest lengths, reserved flags, nesting, allocation and trailing input need one coherent admission contract.
- `plan_entry_ranges` turns a non-32-byte digest into `None`; `verify_arrived_range` succeeds on matching length if neither expected digest nor CRC is present. This is an admission weakness, not a permissible integrity mode for the proposed service.
- An interior `.10d` section CRC detects accidental corruption; it is not adversarial authentication. Whole-entry SHA-256 requires the entire entry and a trusted expected digest. The whole-file CRC requires all bundle bytes.
- The bounded profile requires zero header flags, while the writer exposes a 4096-alignment flag. Agree on supported profiles before claiming every writer output passes every reader.
- The hydration token's parity seal is a publicly computable local consistency check. It is not a signature, MAC, remotely transferable capability, or proof of DID authority.
- `CborLdManifest` carries a supplied `proof_value`, but construction, decoding and geometry-digest comparison do not by themselves verify the signature or issuer authority. The label `VerifiableCredential` is not verification. Its serde/CBOR encoding also needs an explicit mapping to the negotiated semantic profile.

These observations do not imply that surrounding crypto, policy or networking libraries are absent. They identify the assurance boundary of the paths inspected.

## 3. Composition with the existing protocol ecosystem

| System | Existing responsibility | Proposed QBDL relationship |
|---|---|---|
| QLink / QRoute | Bearers and native forwarding | Carry traffic; no bundle-specific routing replacement |
| Identifier Fabric, DID methods, QResolve, QSR | Authority-bearing references and scoped discovery | Resolve resource/provider descriptors; content hashes do not grant permission |
| QSession | Protected sessions, streams, flow control, recovery, migration | Own native transport and negotiated PQ protection |
| Content swarm | Manifest, block layout, provider grants, unordered verified content | Host the bundle-specific object/range/dependency profile |
| Share | Governed offer, acceptance and refusal | Establish transfer scope before protected disclosure |
| QPolicy / QPR | Admission, policy generations, leases, bounded effects | Authorize selection, reserve resources and invalidate stale work |
| QSync | Signed operations, blocks, validation and merge | Publish bundle references/generations; never merge bytes across immutable objects |
| Semantic subscriptions / custody | Scoped updates, gaps, storage commitments | Announce live generations and retain authorized bytes with distinct receipts |
| CSCP | Purpose-scoped connection selection and protection constraints | Exclude prohibited paths before performance ranking |
| LIG | Deliberate legacy Internet interoperability | Own HTTP origin/CDN/provider access and its disclosed trust boundary |
| Commons economics / ontological contracts | Agreements, accounts, finite obligations, settlement | Bind applicable terms without settlement in packet loops |
| HCAI-ANP | Narrow inbound external-agent agreement boundary | Apply to its actor/action scope, not every content download |
| HCF, vault manifests, qapp hosting | Presentation semantics, vault context, installed app hosting | Reference bundles and authorize consumption; no implicit remote execution |

Read the [QDNF overview][qdnf], [QSession services][qsession], [semantic services][services], [CSCP draft][cscp], [Identifier Fabric][identifiers], [did:qi method][didqi], [QSync document][qsync], [LIG specification][lig], [HCAI-ANP][hcai], [HCF document][hcf], [vault manifest][vault], and [qapp loopback protocol][qapp] together. They describe different layers and maturity levels. A locally authored RFC-style draft is not evidence of IETF publication or IANA registration.

Native service opening already binds full service identity, version, purpose, resource, sensitivity, capability decision and optional agreement/reservation references. Extend those semantics. Do not dispatch solely on `q_hash` or another compact identifier. Compact Quin references are lookup aids; full authority records and typed cryptographic commitments remain outside those small fields.

CSCP's exclude-then-rank discipline is essential: a faster disallowed route is never a fallback. HTTP redirects, mirrors, DNS resolution, relay changes and browser fetches can disclose information or alter the trust boundary. They need the applicable path/disclosure checks. The inspected [QDNF fetch gate][fetchgate] distinguishes authorization from revealing an observed locator; payment and group membership do not supply private consent.

## 4. Proposed native service model

### 4.1 Descriptor before bulk data

Use a bounded authenticated **stream descriptor** as the QBDL specialization of the existing content manifest. This is a conceptual record, not a fixed schema. It should bind:

| Information | Purpose |
|---|---|
| Profile, critical features, encoding, cryptographic suite | Unambiguous interpretation and fail-closed negotiation |
| Immutable bundle identity, exact length, generation | Prevent cross-version range assembly |
| Header and index commitments | Authenticate planning metadata before expensive work |
| Entry inventory and verifiable segment map | Select and verify subsets without whole-entry downloads |
| Dependencies, alternatives, initialization, quality semantics | Separate necessary data from optional refinement |
| Provider authority, allowed scopes, expiry | Separate byte suppliers from publishing authority |
| Publication, confidentiality, policy references | Govern metadata, payload, caching and redistribution |
| Pinned semantic bundle and applicable agreements | Bind meaning and duties without mutable context fetches |
| Availability, retention, resume, live-generation rules | Make missing data and continuity explicit |
| Resource estimates and negotiated ceilings | Bound parser, crypto, decoder, GPU and storage work |

A detached descriptor lets existing immutable QBDL objects acquire a stronger delivery envelope without rewriting every entry. It avoids an embedded manifest circularly committing the exact hash of the bundle containing itself. An embedded profile would need exact exclusion rules or a two-stage commitment with vectors.

The descriptor requires a trusted bootstrap: an authorized resource reference, verified provider/controller chain, or already pinned commitment. A hash supplied by the same untrusted server as its payload is not an authentication root. Policy decides when a known content commitment is sufficient for a public read and when publisher authority is additionally required.

### 4.2 Native exchange and lifecycle

Proposed abstract operations are offer/describe, select, reserve/open, request ranges, transfer proof-bearing segments, accept/reject, cancel, resume, and close/reconcile. These are responsibilities, not allocated opcodes or replacements for Share/content-service operations.

```text
Resolve permitted resource and provider
  -> admit Share/content intent and PQ service profile
  -> verify bounded descriptor and pinned interpretation
  -> select authorized dependency closure and reserve resources
  -> request segments over QSession streams
  -> verify proofs, decrypt, validate embedded format
  -> admit bounded decode or compute; report explicit completion state
  -> checkpoint / resume / cancel / close and reconcile
```

Each operation binds object, profile, authority/policy generation, cancellation epoch, deadline, quotas and receipt identity. Out-of-order data populates reserved ranges only. Late completion cannot revive cancellation or write into a reused slot. Restart, rekey, migration and provider replacement must not change the committed object or create duplicate economic acceptance.

QSession owns transport reliability and congestion control. The bundle profile owns semantic priority and dependencies. Do not add competing retransmission loops under reliable streams. The existing reliable-carrier proposal explicitly controls duplicate packet recovery; it cannot be inferred because one hop happens to be reliable. [QSession][qsession]

### 4.3 Three forms of streaming

**Immutable progressive retrieval:** obtain enough of a fixed bundle for a useful verified partial result. This is the first conformance target.

**Live generation publication:** publish successive immutable descriptors/bundles with sequence, predecessor, availability window, dependency cut, discontinuity and finalization semantics. Existing subscription/sync machinery announces them. Missing retained history produces an explicit gap, never false continuity.

**Unfinalized producer streaming:** consumption while a bundle is still being written needs a separately specified framing/checkpoint or format extension. QBDL v1's finalized header/footer must not be described as already providing it. Prefer immutable incremental units first; benchmark whether an append-oriented profile is warranted.

## 5. Post-quantum cryptography as a design requirement

### 5.1 Threat model and profile ownership

PQ protection is a target requirement, not an optional label. It covers long-lived confidentiality against capture-now/decrypt-later attacks, active authentication, content/policy commitments, stored capabilities, update authority and retained evidence. QPU participation does not itself provide these protections, and PQ cryptography does not require clients to own QPUs.

Reuse the [QPR PQ profile][pq] rather than inventing bundle cryptography. Its proposed choices are fresh X25519 plus ML-KEM-768 key establishment, paired ML-DSA-65 and Ed25519 proofs with both required, typed SHA-384 commitments, purpose-separated HKDF/HMAC-SHA-384, and ChaCha20-Poly1305 traffic protection. SLH-DSA is separately profiled for infrequent root/recovery evidence. These repository design choices require integration and independent review; they do not establish what every running client negotiates.

ML-KEM establishes shared secret material, rather than encrypting bulk files directly. ML-DSA and SLH-DSA are signature schemes, distinct from symmetric payload encryption. Pin standards, parameter sets, implementations and applicable errata in release evidence. [NIST FIPS 203](https://csrc.nist.gov/pubs/fips/203/final), [FIPS 204](https://csrc.nist.gov/pubs/fips/204/final), [FIPS 205](https://csrc.nist.gov/pubs/fips/205/final).

The authority chain matters as much as the session. The current `did:qi` method describes classical Ed25519 proofs and SHA-256 derivation. A PQ session cannot automatically confer PQ assurance on that root or old authorizations. Define an explicit upgrade/binding path and its claim without reinterpreting old bytes or assuming a hash of an old hash strengthens the original commitment.

### 5.2 Trust boundaries for review

| Threat or boundary | Required distinction |
|---|---|
| Malicious mirror or cache | Untrusted delivery may supply valid bytes; authority and object commitments come from verified evidence |
| Compromised publisher/controller | Signatures establish authorized key use, not scientific truth or benign content; key compromise and revocation need explicit handling |
| On-path or future quantum attacker | Protect session secrecy, negotiation, authority chains and durable commitments under the selected PQ profile |
| Authorized but overreaching recipient | Access is scoped; cryptography cannot force deletion of plaintext already disclosed |
| Hostile metadata or embedded code | Authenticate before expensive work, then still validate formats, budgets and execution authority |
| Gateway or provider termination | End-to-end claims stop at actual cryptographic endpoints; custody and plaintext processing need separate grants |
| Stale or partitioned economic replica | Resource access, reservation, accepted delivery and settlement finality remain distinct; uncertainty cannot mint authority |

The review should state which parties are trusted for content authority, policy, discovery, time, custody, computation and settlement. Those roles need not belong to the same operator. Do not assume that a trusted transport endpoint is also a trusted result validator or policy interpreter.

### 5.3 Integrity layers and the truncated-hash bug

CRC-32C, a full digest, a Merkle/range proof, an authorized signature and encryption serve different purposes. APIs and documentation must preserve those distinctions.

**The historical truncated BLAKE3 checks in EMF/M64 are bugs.** They are not approved bandwidth optimizations or standard integrity features. Legacy 16-byte or 12-byte prefix comparisons must not be described as full BLAKE3 verification or Bao proof verification. Current repair work stores and checks a full 32-byte BLAKE3 digest in a versioned/identified chunk encoding. A full digest still is not a Bao proof, signature or trusted commitment to a partial range. [Chunk integrity repair][integrity]

For the new PQ stream profile, use the full typed commitments required by QDNF. A SHA-384 commitment over exact content or properly specified tree leaves should not merely wrap a legacy SHA-256/BLAKE3 identifier when the claim requires stronger collision assurance throughout. Existing QBDL SHA-256 fields can remain legacy format checks while the PQ descriptor independently authenticates exact bytes and ranges. Old magnets and compact identifiers may remain locators; they are not the new security root.

For independent segment consumption, choose between an authenticated digest list and a Merkle/range-proof construction. Lists simplify verification but grow with segment count. Trees can reduce metadata and support selective retrieval, at the cost of proof shape, leaf framing, final-leaf rules, memory and verification complexity. Bao is a candidate only under an explicitly reviewed algorithm/profile; the name alone neither supplies proofs nor satisfies QDNF's SHA-384 commitment requirement.

Bind proof context to protocol version, immutable object, entry/segment identity, offset, exact length and payload commitment. Separate leaf/internal-node domains with unambiguous encoding. Test reorder, splice, truncation, duplicate leaves, empty objects and cross-profile substitution. A proof for one bundle or range cannot authorize another.

### 5.4 Encryption, custody and HTTP edges

Session encryption protects the path between its actual endpoints. It does not prevent an ordinary CDN, HTTP gateway or QPU provider from reading plaintext it terminates. Private store-and-forward delivery needs a reviewed object-encryption profile as well as the PQ session.

Evaluate independently authenticated encrypted segments with unique key/nonce domains and associated data binding object generation, segment identity, length and profile. Define whether commitments cover ciphertext, plaintext or both. Ciphertext verification may precede decryption; plaintext checks follow successful AEAD. Never release partially authenticated plaintext. Do not invent an ad hoc KEM combiner or reuse nonces to preserve deterministic output.

Content keys, recipient wraps, key rotation and revocation are separate from the immutable artifact. Specify authentication of changed recipient envelopes, opaque cache storage and visible metadata. Re-encryption normally creates a new ciphertext identity. Revocation can stop future key release or access; it cannot recall disclosed plaintext.

For HTTP compatibility, retain native PQ artifact envelopes and grants where endpoints support them. If a legacy browser/origin cannot meet the required profile, report that limitation and refuse protected delivery or use a separately authorized compatibility representation. Ordinary HTTPS alone is not evidence of PQ protection. Compatibility must never silently downgrade required PQ confidentiality or authentication.

Budget large keys, proofs and transcripts before parsing or verification, including fragmented handshakes and denial paths. Bind algorithm negotiation and policy to the transcript. Failure of one required paired proof is failure of the profile, not permission to accept the other. Version old/new records explicitly and define anti-rollback, key rotation and evidence renewal.

## 6. Segmentation, dependencies and adaptation

### 6.1 Follow usable boundaries

| Payload | Useful units to investigate | Dependencies and cautions |
|---|---|---|
| `.q42` | Authorized snapshots, blocks and semantic subsets | Whole-volume transfer can disclose restricted graphs; projection identity is not file identity |
| `.10d` | Metadata/table, ontology/procedural data, proxies, geometry and fields | Authenticate descriptors; preserve skeleton/material/UV relationships |
| `.emf` | Directory and spatial/spectral/time bricks | Preserve units/indexing; preview is not the durable scientific field |
| `.m64` / `.e64` | Directory, metadata/LUT references and matter chunks | E64 is a compatibility identity, not EMF; mode/codec govern interpretation |
| `.p64` | Model metadata, tokenizer and supported tensor/page groups | Decoder dependencies and memory admission determine actual usability |
| HCF / ordinary media | Presentation dependencies, accessible alternatives, timed segments | Preserve language, captions and semantics; negotiate codecs separately |
| Reusable solver/QPU result | Generic problem, result, validation/provenance and policy evidence | Check applicability and quality before rehydrating local semantics |

Choose transport aggregation independently of semantic segmentation. Several small authorized segments may share a request while remaining independently verifiable. A large segment may transfer in bounded pieces but cannot be consumed before its required verification boundary is complete. Coalescing must never fetch or disclose unauthorized gap bytes.

### 6.2 Adapt on more than throughput

Selection should consider purpose, dependency readiness, memory, decode time, CPU/GPU/QPU capability, energy allowance, accessibility, scientific precision, network conditions and agreed cost. Evaluate hard authorization requirements before preferences. Estimate benefit per admitted resource cost without inventing a universal quality score.

LOD levels and numerical precision are not always interchangeable. Distinguish display proxies from analysis-grade data, quantization from lossless compression, alternate biological stages from detail levels, and changed model weights from equivalent encodings. Switching requires an explicit compatibility relation, initialization dependencies and a legal switch point. A cheap representation must not silently change the scientific answer.

Existing three-tier priorities are a useful implementation seed, not a complete dependency graph or measured latency guarantee. Use bounded DAG traversal, cycle rejection, maximum closure size and deterministic tie-breaking where appropriate. Required dependencies must not starve behind refinements. Cancellation reclaims queued work while respecting in-flight ownership.

## 7. HTTP backward-compatible delivery binding

This binding translates admitted native content operations to HTTP; it does not define a second authority model. Native resolution and delivery must work without it. Ordinary HTTP hosts can serve explicitly public immutable bundles, while governed gateways need declared authority, disclosure scope and receipt semantics.

HTTP byte ranges and partial responses are defined by RFC 9110. Responses use inclusive end offsets; the core planner uses offset plus length. Validators and `If-Range` support consistency, but an entity tag is not a cryptographic authority proof. HTTP/3 maps HTTP semantics to QUIC; it does not replace QDNF's content or governance semantics. [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html#name-range-requests), [RFC 9114](https://www.rfc-editor.org/rfc/rfc9114.html), [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html).

Proposed binding requirements:

1. Pin the native object commitment and exact length before assembling ranges. Validate responses against that identity and requested interval.
2. Begin with single-range requests. Check conversions, overflow, zero length and bounds before forming `bytes=start-end`. Reject multipart responses unless bounded parsing is explicitly supported.
3. Require a byte-preserving representation. Refuse transparent whole-object transformations that invalidate offsets. Internal compression belongs to independently described segments and bounded decoder profiles.
4. Define handling of `206`, ignored-range `200`, `416`, redirects, partial bodies, changed validators and timeouts. A `200` response is not the requested partial segment; full-download fallback needs explicit policy and size limits.
5. Provider retries must stay within permitted authority/path scope and the same immutable commitment. Do not forward credentials to arbitrary redirect targets.
6. Cache keys include content generation, encryption/security profile and privacy partition. Content equality does not grant cross-user disclosure. Negative responses must not become existence or entitlement oracles.
7. Browser integration specifies CORS/exposed response metadata, worker/OPFS quotas, cancellation and consumer cross-origin-isolation requirements. Test actual browser behavior rather than assuming native transport/header control.
8. Preserve native completion meanings: received bytes, accepted proof, consumer admission and economic acceptance remain separate events.

Cache controls guide compliant caches; they are not confidentiality or remote erasure guarantees. Apply publication/custody policy alongside HTTP caching. [RFC 9111](https://www.rfc-editor.org/rfc/rfc9111.html).

### 7.1 Relationship to HLS and DASH

HLS supplies media playlists, duration/timeline semantics, byte-range segments and variant selection. DASH supplies presentation/representation models, segment addressing and availability/timing rules. Useful lessons include explicit initialization, aligned switching, bounded availability windows and discontinuities. [HLS RFC 8216](https://www.rfc-editor.org/rfc/rfc8216.html), [DASH-IF timing model](https://dashif.org/Guidelines-TimingModel/).

Use HLS/DASH adapters for audiovisual presentations when that improves player interoperability. An adapter exposes suitable encoded media and maps presentation timing, accessibility and variants. It does not make arbitrary QBDL bytes playable by a stock HLS/DASH player. Scientific volumes, weights and semantic dependencies remain native content-service concerns. HTTP range support alone does not establish HLS/DASH conformance.

## 8. Permissive Commons, consent and publication

### 8.1 Preserve existing distinctions

The Commons is governed sharing, not a blanket public-data designation. [Webizen Permissive Commons][commons] distinguishes private Selfhood, governed artifacts, custodianship and reusable knowledge. The [publication implementation][publication] distinguishes `Restricted`, `ProjectInternal`, `PublicNotForRedistribution` and `PublicRedistributable`; only the last permits an affirmative public redistribution declaration. It also distinguishes Commons, gated Commons, unmarked/local, Sanctuary and mixed fail-closed volumes.

For QBDL, propose an aggregate publication decision over **all disclosed bytes and metadata**: index keys, filenames, sources, previews, thumbnails, provenance, dependencies and sidecars. A Commons declaration on one entry cannot authorize a private sibling. Mixed bundles need a separate authorized projection or split publication; hiding an index row while serving the same unrestricted object is insufficient.

Distinguish discovery, access, copying, redistribution, derivation, execution, custody, evidence disclosure and spending. A signature, subscription, social relationship, entitlement or payment does not imply all of them. Apply [socially defined protection][protection] without requiring disclosure of protected classifications. Private lookup should avoid broadcasting content interests, locations and relationship graphs.

HCAI-ANP's operator acceptance and accountability mechanisms apply within its stated external-agent scope. They do not substitute for authorization or require an identity-heavy negotiation for every public artifact read.

### 8.2 Contracts bind bytes and interpretation

Follow [ontological contracts][contracts]: bind exact contract bytes and the complete pinned context, ontology, shapes, rules and compression-table closure. A mutable vocabulary URL cannot change obligations after acceptance. Bounded cold acquisition may retrieve missing dependencies; packet processing cannot dereference arbitrary contexts or run unbounded reasoning.

Separate syntax, semantic constraints, signature/authority validity, policy applicability, acceptance and reservation. SHACL success does not prove truthful evidence or spending authority. Unknown critical terms prevent acceptance; inert extensions require the declared profile. Generic CBOR serialization is not proof of conformance to the negotiated CBOR-LD contract profile. [CBOR RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html).

The entity manifest's `CommonsObligation` provides useful compatibility data: production cost, ROI field, accumulated pool, carriage rate and perpetual-fee prohibition. Its helper currently passes a fixed cap argument. This is not a substitute for a signed, authoritative multi-provider obligation ledger, operation classification, units or accepted funding. Do not standardize that helper's cap as universal economics. [Manifest][manifest], [socioeconomic helpers][socioeconomics]

### 8.3 Resource accounts and finite obligations

Apply [Commons and resource economics][economics] and [finite project compensation][finite] as the current design direction. Separate operating resources, accepted agreements, creation-recovery obligations and settlement. Streaming introduces no mandatory token, blockchain, wallet, artificial proof-of-work, advertising exchange or payment provider. Gifted, sponsored, pooled, reciprocal and paid provision remain possible under accepted terms.

Measure bytes, storage, verification/decode work, scoped time and energy using typed quantities and evidence status: measured, estimated or unknown. Unknown energy is not zero. GPU work, CPU time, QPU shots and money are not interchangeable without explicit valuation. Reserve aggregate limits before bounded work, including concurrent sessions and mirrors. Packet paths use compiled handles rather than contacting settlement systems.

Recovery of a named finite creation obligation stops at its agreed target. Another provider, different segmentation, a representation switch, resumption or a new endpoint must not reset that target. Personal humanitarian exemptions and represented-principal rules apply to the operation, not a permanent label on a person. Ongoing hosting costs are separately agreed; they cannot silently reopen discharged creation charges.

If only 7 agreed recovery units remain, concurrent mirrors cannot each reserve 7. They need shared authoritative remaining-cap accounting or bounded allocations totaling at most 7. An offline replica with stale state cannot promise exact global settlement: define whether it serves under gift/subsidy allowance, waits, or uses an accepted bounded reservation. Do not charge protected users merely because reconciliation is unavailable.

Receipt identities survive reconnects and bind agreement, operation, accepted segment and obligation. Retransmission can consume resources, but duplicate delivery must not become duplicate accepted creation recovery. Checked arithmetic and carried fractional rounding prevent fee amplification through request splitting. Transport ACK, verified delivery, useful computation, custody and payment finality are different evidence.

## 9. Reusable computation and the QPU Commons use case

This is an application of the fabric and Commons model. It is already described in [Webizen Permissive Commons, sections 2.4–2.5][commons]: remove local semantic metadata from a mathematical structure, identify it, and reuse prior solutions instead of repeatedly buying scarce quantum computation. The user's intended order is explicit:

```text
Local problem -> admitted solver cascade
  -> unresolved residual requiring escalation
  -> safe generic mathematical formulation
  -> scoped Commons lookup for an applicable historical solution
       -> hit: fetch evidence/result, validate, rehydrate locally
       -> miss/insufficient evidence: authorize new QPU work
          -> validate result -> review publication -> reusable Commons artifact
```

### 9.1 Preserve meaning when generalizing

Removing names is insufficient. Define a versioned canonical mathematical representation covering variables, coefficients, units, domains, constraints, objective direction, tolerances and formulation/compiler profile. Equivalent variable permutations need a reversible mapping; normalization must record how results map back. Keep semantic-to-variable mappings local unless a separate disclosure grant permits otherwise.

Identical connectivity with different weights is generally a different optimization problem. Positive objective scaling may preserve an optimum while changing thermal sampling or other requested properties. Graph-isomorphism hints, approximate-neighbor search and shared topology can locate candidates, but cannot establish answer equivalence alone. Parameterized families permit reuse only with a validated applicability domain and substitution witness. Bound canonicalization work; do not assume general mathematical equivalence is cheaply decidable.

Use full typed PQ-profile commitments to canonical problems and their interpretation. Distinguish problem key, result artifact identity and original private query. Canonicalizer changes create a new equivalence namespace unless a reviewed mapping exists. Low-entropy hashes and stripped numerical matrices can still expose private structure; assess disclosure before public lookup even when names are absent.

### 9.2 Lookup does not establish novelty or correctness

Search admitted local caches and authorized Commons catalogs before QPU submission. Distinguish exact applicable hit, candidate needing validation, known in-flight work, insufficient-quality result, no result within searched scope, and lookup unavailable. A miss is not proof nobody has solved the problem. Record catalog generation, authority, coverage and freshness without disclosing the original private context.

A result binds the generic problem, formulation/version, assumptions, solver/backend, method, quality target, achieved metrics, validation evidence, provenance and publication/use terms. Quantum provenance does not prove global optimality. Distinguish feasible solution, sampled estimate, bounded approximation, certified optimum and unverified claim. Recompute feasibility/objective where possible; require the actual certificate when certification was requested. Preserve relevant uncertainty, shots/calibration and limitations.

QBDL packages generic problems, results and evidence for partial or offline retrieval; QDNF discovery and semantic indexes locate candidates; QSync publishes accepted references and updates. Download receipts do not prove solution correctness. Rehydration is a separate local semantic operation and must not execute arbitrary remote instructions.

### 9.3 New work, contribution and release

Before necessary remote work, reserve provider capacity, disclosure, shots/runtime and accepted cost. Deduplicate concurrent jobs through reviewed leases/reservations. Ambiguous timeouts require provider/job reconciliation before resubmission, not automatic second execution or billing. Unavailability cannot silently change the requested method or accuracy.

Publish only reviewed generic results and permitted provenance. Permission to solve a private problem is not permission to publish it. Attribute useful contributions, validation and stewardship under accepted terms. Reuse avoids unnecessary QPU work; it must not bill each consumer as though a new quantum job occurred. Finite recovery follows the same shared cap and humanitarian rules as other Commons artifacts.

The inspected tree includes [solver/QPU types][qpu], [QUBO formulation and rehydration][qubo], [client pipeline][qpupipeline] and [provider dispatcher][qpudispatch]. The dispatcher inspected attempts configured providers and has classical fallback; that path alone does not establish the complete solver-first, historical-Commons-first lifecycle above. Trace and integrate the defined Commons architecture through actual entry points rather than declaring the QPU system absent or inventing a second dispatcher. No provider call or QPU job was executed for this brief.

## 10. Runtime, storage and failure semantics

Honor QPR's two execution tiers: bounded cold descriptor/policy/proof preparation and allocation-free hot state transitions. Reserve wire buffers, decoded metadata, semantic expansion, crypto scratch, dependency traversal, decompression, GPU upload, in-flight I/O and partial storage. Input size alone does not bound output or verification work. [QPR API][runtime]

Negotiate ceilings for descriptors, entries, key lengths, proof depth/bytes, concurrent ranges, dependencies, decompressed output, work/fuel, bandwidth and disk. Existing bounded-reader defaults describe a local parser profile, not universal streaming limits: 128 MiB full-file ceiling, 1 MiB index, 4096 entries. Large P64 bundles need a deliberate streaming profile with checked 64-bit lengths and bounded working sets, not removal of limits. [Reader limits][limits]

Sparse storage distinguishes holes from verified zeros. Checkpoints bind object generation, proof profile, verified range set, storage durability and policy epoch. Revalidate authorization on resume; verify persisted bytes as required. An unchanged URL is insufficient. Atomic publication exposes only verified, dependency-ready objects. Garbage collection respects active leases, custody promises and evidence holds.

Define safe outcomes for unsupported critical features, policy denial, pending authority, stale generation, invalid proof, format rejection, dependency gaps, exhausted quotas, unavailable providers, cancellation and ambiguous settlement. Sensitive details stay local; remote errors cannot disclose protected existence or contents. Exhaustion is a controlled state, not permission to weaken verification or bypass policy.

## 11. Privacy, evidence and observability

Encryption still exposes some combination of size, timing, range patterns, provider choice and identifiers. Spatial queries can reveal attention or location; model requests can reveal work; mathematical lookups can reveal private problems. Evaluate partitioned caches, minimized descriptors, scoped discovery and optional padding against costs. Do not claim perfect anonymity, zero knowledge or unlinkability from encryption or hash addressing alone.

Useful measurements include time and bytes to first verified useful state, cancelled/wasted bytes, verification/decode time, peak working set, proof overhead, cache reuse, dependency stalls and policy denials. Collect them without a default behavioral dossier. Report hardware, network, object and policy conditions. This brief asserts no unmeasured latency or energy savings.

Apply [electronic evidence and retention][evidence] and [consent/accountability ADR 0011][adr11]. Diagnostics, operational receipts and selected evidence have different retention/disclosure rules. Preserve interpretation and authority context for retained receipts, including failures and gaps; avoid logging payloads or secrets by default. A preservation hold is scoped authority, not permission to fetch all related private content. Digests cannot reconstruct discarded evidence.

## 12. Conformance and adversarial review programme

These are proposed release tests, not claims that a streaming implementation has passed them.

| Area | Required evidence |
|---|---|
| Native independence | Discovery, authorization and QBDL transfer through the admitted QDNF profile without HTTP/DNS/CDN dependence |
| Binding parity | Native and HTTP bindings reach equivalent object, range, policy and completion decisions on shared fixtures |
| PQ chain | Negotiation, paired proofs, authority-root upgrade, exact-byte commitments, rotation and anti-downgrade vectors |
| Metadata admission | Oversized/nested CBOR, duplicate fields/keys, bad digest lengths, overflow, overlap, reserved bits, critical extensions, truncated bodies |
| Partial integrity | Altered proof tail, absent proof, cross-entry splice, generation replay, duplicate/reordered ranges, forged header/index, final-leaf cases |
| Confidentiality | Ciphertext tampering, nonce separation, envelope substitution, denied keys and no plaintext before authentication |
| Governance | Mixed Commons/Sanctuary objects, private metadata, expired/revoked grants, paid-but-denied requests and stale policy |
| Runtime | Aggregate limits, reservation races, decompression bombs, cancellation, slot reuse and repeated hostile misses |
| HTTP edge | Ignored Range, malformed Content-Range, changed objects, transformations, redirect leakage, partial caches, browser limitations |
| Live/resume | Missing generations, retention gaps, stale checkpoints, crash recovery, mirror changes and false continuity rejection |
| Economics | Duplicate receipts, request splitting, shared-cap races, humanitarian exemptions, discharged obligations, ambiguous settlement |
| Computation reuse | Equivalent renamings, unequal coefficients, invalid normalization, poisoned results, inadequate quality, scoped misses, duplicate jobs |
| Embedded formats | Reader/version compatibility, alignment, dependency closure and legacy-hash migration without silent downgrade |

Use independently produced positive/negative vectors, parser fuzz/property tests, deterministic state-machine traces, native/WASM tests and resource instrumentation. Include an independently implemented peer before claiming interoperability. Primitive PQ vectors alone do not validate the composed protocol or its authorization logic.

## 13. Implementation sequence and ownership

| Stage | Deliverable and exit condition |
|---|---|
| A — Architectural agreement | Content-swarm boundary, native/HTTP roles, PQ requirement, policy/threat model and initial workloads |
| B — Descriptor and admission | Bounded exact encoding, pinned semantics, authenticated metadata/proofs; no unchecked-range admission |
| C — PQ integration | Reviewed session/authority/object protection, migration/downgrade tests; no protected classical-only path |
| D — Native immutable streaming | Service composition, dependencies, reservations, verified partial hydration, cancellation and resume |
| E — HTTP compatibility | LIG/HTTP adapter using the same policy gates and fixtures; browser/cache tests; native independence |
| F — Commons and reuse | Agreement/receipt integration, publication aggregation, finite-cap reconciliation, reusable-result discovery |
| G — Extensions and release | Live generations, optional media adapters, independent interoperability, measurements, security review, registry allocation |

These are dependencies, not a suggestion to postpone PQ until after deployment. Stages B–D establish PQ assurance for the new native profile. HTTP compatibility is assessed against that baseline.

Ownership stays with existing layers: QualiaDB owns codecs, policy integration and native service behavior; QDNF/QPR owners own sessions, connection semantics and cryptography; producers such as Synth and Hypermedia emit descriptors/representations; render/inference consumers own decoding and application readiness. Crypto primitives remain with existing libraries. HTTP/HLS/DASH adapters must not become parallel policy engines.

Initial scope: immutable retrieval, authenticated descriptor/segments, native enforcement, private/public distinctions and bounded resume. Live producer framing, universal equivalence solving, arbitrary remote execution and new cryptographic constructions require separate reviewed profiles.

## 14. Advisor decision register

| ID | Question | Recommended starting position |
|---|---|---|
| A01 | Content-swarm extension or new service? | Extend existing service; justify a new identity only for genuinely distinct lifecycle/negotiation semantics |
| A02 | Descriptor bootstrap and binding? | Detached authenticated descriptor rooted in an authorized reference; exact header/index/object commitments |
| A03 | PQ suite and authority migration? | Reuse explicit QPR PQ profile; resolve classical root dependencies before end-to-end PQ claims |
| A04 | Digest list or tree proof? | Compare real-object costs; typed full SHA-384 commitments for the target PQ profile |
| A05 | Encrypted object boundary? | Independently authenticated segments and reviewed recipient/key lifecycle |
| A06 | HTTP-only endpoint behavior? | Public legacy access or supported artifact envelope; no protected-profile downgrade |
| A07 | Valid representation switch? | Explicit semantic/quality relation and initialization closure, not filename heuristics |
| A08 | Mixed-rights publication? | Authorize full disclosure closure or generate a separate permitted object |
| A09 | Budgets and receipts? | Aggregate admission before work; separate verified delivery, application, custody and settlement |
| A10 | Finite recovery across mirrors? | Authoritative shared cap or bounded allocations; stale replicas cannot invent spend authority |
| A11 | Valid reusable-solver hit? | Versioned canonical problem, applicability and quality evidence, private rehydration mapping |
| A12 | Live updates? | Immutable generations over subscriptions/sync first, explicit gaps/discontinuities |
| A13 | Legacy formats? | Explicit compatibility/migration; no truncated-hash assurance or reinterpretation of bytes |
| A14 | Release evidence? | Shared vectors, independent native peer, PQ composition review, measured resource bounds and policy tests |

The requested expert response should supply a preferred architecture, rejected alternatives with reasons, threat-model review, concrete profile choices, ownership/dependency changes and prioritized issues. Wire schemas and registry allocations should follow those decisions.

## 15. Source and status notes

Primary local sources are linked where used. External sources establish cited transport, encoding and cryptographic concepts; they do not certify this proposal. Older Commons/HCF narratives contain broad claims about Bao streaming, privacy or historical economics that require checking against code and newer scoped profiles. This brief preserves ecosystem intent without adopting unsupported guarantees.

The separate file-format integrity repair is development work, not evidence that this streaming service exists. On 12 November 2026 the focused container conformance harness passed 61 tests, including full-digest mutation and explicit legacy admission cases. The actual producer integration suites also passed 7 electromagnetics and 5 geodesy tests. These results concern EMF/M64/E64/CRC and their producer paths only; it does not establish QDNF streaming, PQ interoperability or deployed-client compatibility. The six-format standards programme remains separately tracked in the [work checkpoint][checkpoint].

[format]: ../../../crates/qualia-core-db/src/bundle/format.rs
[loader]: ../../../crates/qualia-core-db/src/bundle/streaming_loader.rs
[limits]: ../../../crates/qualia-core-db/src/bundle/reader_limits.rs
[hydration]: ../../../crates/qualia-core-db/src/bundle/streaming_hydration.rs
[workers]: ../../../crates/qualia-core-db/src/governance/worker_pool.rs
[manifest]: ../../../crates/qualia-core-db/src/bundle/cbor_ld_manifest.rs
[variants]: ../../../crates/qualia-core-db/src/bundle/variant_matrix.rs
[integrity]: ../../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs
[publication]: ../../../crates/qualia-core-db/src/q42/volume/publication.rs
[socioeconomics]: ../../../crates/qualia-core-db/src/modalities/socioeconomics.rs
[qdnf]: qualia-decentralized-network-fabric/README.md
[qsession]: qualia-decentralized-network-fabric/qsession-and-services.md
[services]: qualia-decentralized-network-fabric/semantic-peer-services.md
[runtime]: qualia-decentralized-network-fabric/peer-runtime-api.md
[cscp]: ../../standards/ietf/draft-webcivics-cscp-00.md
[identifiers]: qualia-decentralized-network-fabric/identifier-fabric-integration.md
[didqi]: did-qi-method.md
[qsync]: qualia-sync-protocol.md
[lig]: qualia-decentralized-network-fabric/legacy-internet-gateway.md
[fetchgate]: ../../../crates/qualia-core-db/src/net/qdnf/session/fetch.rs
[hcai]: hcai-agreement-negotiation-protocol.md
[hcf]: hypermedia-content-format-hcf.md
[vault]: qualia-vault-manifest.md
[qapp]: qualia-qapp-loopback-protocol.md
[pq]: qualia-decentralized-network-fabric/post-quantum-security.md
[commons]: ../webizen_permissive_commons.md
[contracts]: qualia-decentralized-network-fabric/ontological-contracts.md
[economics]: qualia-decentralized-network-fabric/commons-and-resource-economics.md
[finite]: qualia-decentralized-network-fabric/finite-project-compensation.md
[protection]: qualia-decentralized-network-fabric/socially-defined-protection.md
[evidence]: qualia-decentralized-network-fabric/electronic-evidence-and-retention.md
[adr11]: ../adr/0011-human-centric-consent-accountability-and-disposition.md
[qpu]: ../../../crates/qualia-core-db/src/solvers/qpu/mod.rs
[qubo]: ../../../crates/qualia-core-db/src/qubo_compiler.rs
[qpupipeline]: ../../../crates/qualia-client-core/src/qpu_pipeline.rs
[qpudispatch]: ../../../crates/qualia-client-core/src/qpu_dispatcher.rs
[checkpoint]: file-formats-standards-work-in-progress.md
