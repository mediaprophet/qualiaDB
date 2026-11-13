# QBDL streaming architecture and interface contract

Revision 1, 12 November 2026. Decisions below are implementation targets. Exact CBOR keys, domain bytes and registry allocations are frozen by QBS-01/03/05/10 with independent vectors before interoperable production code. Working names are not published registrations.

## 1. Composition and typed boundaries

```text
QResolve / pinned authority → Share + QPolicy → authenticated descriptor
  → dependency closure + QPR/consumer reservations → carrier-neutral requests
  → native QSession | HTTP/LIG | WebSocket | WebRTC data channel
  → range proof → optional AEAD → format admission → policy recheck
  → verified storage / Etalon consumer commit → typed receipts
```

QSession owns native connection protection, recovery and congestion. CSCP excludes prohibited paths before ranking eligible ones. HTTP/WebSocket/WebRTC are compatibility carriers, not alternative permission systems. Native qualification must not require them. Parent reliable-carrier contracts control whether QSession packet recovery is bypassed; do not tunnel a full lossy-packet recovery loop over TCP or reliable SCTP without that explicit mode.

Define these interfaces as bounded Rust contracts, with caller-owned buffers and nonallocating errors. Names are proposed, not claims that these exports already exist:

| Interface | Input → result; owner |
|---|---|
| `admit_descriptor` | Bounded encoded body/proofs, trust context, limits → `VerifiedDescriptorHandle`; crypto + bundle |
| `admit_index` | Authenticated header/index slices, remote u64 extent, limits/scratch → validated entry view; bundle |
| `compile_selection` | Descriptor, requested semantics, policy, resource estimates → immutable `StreamPlanHandle`; bundle/policy |
| `reserve_transfer` | Plan, consumer requirements, epoch/deadline → non-cloneable `TransferLease`; QPR |
| `request_leaves` | Lease, object/entry/leaf IDs, priority, credit → bounded request; service/carrier adapter |
| `verify_leaf_into` | Exact bytes, typed proof/root/context, reserved output → `VerifiedLeaf`; proof owner |
| `decrypt_leaf_into` | Verified ciphertext, scoped key handle, context → private authenticated plaintext lease; crypto |
| `admit_segment` | Required verified leaves and authenticated metadata → format-specific admitted view; codec owner |
| `commit_consumer` | Admitted closure, current policy/cancel epoch, consumer reservation → accepted artifact/receipt; consumer |
| `checkpoint` / `resume` | Verified bitmap, storage generation, authority state → bounded durable checkpoint / revalidated plan |

Raw bytes cannot manufacture verified handles. Handles contain slot/generation identity and remain process-local; network tokens are verified records compiled into handles, never serialized pointer/parity capabilities. The service treats `Transferred`, `VerifiedCiphertext`, `AuthenticatedPlaintext`, `StructurallyAdmitted`, `PolicyAccepted`, `ConsumerCommitted`, `CustodyAccepted` and `Settled` as distinct facts. Failure/cancellation releases reservations once; late completions cannot reuse a slot after its generation changes.

## 2. Immutable descriptor and metadata pages

QASD is detached from QBDL. Split immutable content identity from renewable publication proofs, provider locators, access grants and recipient wraps. The immutable body has schema/profile and critical-feature IDs; exact QBDL length/version/header/index SHA-384; entry-page commitments; representation roots; leaf geometry; generation/predecessor; semantic-closure and policy references; bounded dependency-page commitments; and required consumer/resource profiles. Unknown critical fields fail. A new proof/locator does not change content identity; a changed content body does.

Use deterministic CBOR with definite lengths, minimal integers, fixed key ordering, no duplicate map keys, no floats in security/range fields and explicit text rules. Reject trailing data and arithmetic overflow. Inner legacy QBDL CBOR retains its existing encoding. Semantic CBOR-LD contracts are pinned artifacts referenced by digest; plain Serde CBOR is not automatically CBOR-LD. No live context/ontology URL fetch runs during admission.

Root body and complete signature envelope each have separate byte limits. Inventory/DAG pages are independently committed, fetched and checked before use. Bound cumulative pages/edges/text bytes, not just each page. Detect cycles, unresolved dependencies, duplicate identities and incompatible alternatives. Do not download an unbounded footer to learn its size: use authenticated root bounds and the fixed header first.

Logical entry IDs are opaque and stable within the content context. Bind exact offsets, lengths and inner-format version to roots. Index/header promises are cross-checked against actual bytes; publisher mistakes are refusals. Public raw QBDL must be public as a whole, including keys, metadata, gaps and sidecars. Mixed-rights source bundles require a separately compiled public projection or private per-entry representation with confidential inventory. Hiding descriptor rows while exposing the raw origin is forbidden.

## 3. PQ proofs, authority and construction order

Reuse the [QPR profile](../../post-quantum-security.md). Initial publication proof is **two COSE_Sign1 objects** over the same exact binding object and external-AAD policy, with both signatures required. The binding names both key references/algorithms, controller epoch, purpose/audience, profile and exact body digest. Freeze ML-DSA-65 `-49` and fully specified Ed25519 `-19`; do not newly emit the advisor's polymorphic `-8`. These are from [RFC 9964](https://www.rfc-editor.org/info/rfc9964/) and [RFC 9864](https://www.rfc-editor.org/rfc/rfc9864.html). Their signing structures and primitive context rules must match vectors, not an existing ad hoc context wrapper.

Cold admission verifies paired-key authority, validity/revocation, audience and policy generation. A self-signed PQ key delivered by an untrusted provider is not a bootstrap. R1 supports explicit trusted full paired-key pins and verified PQ controller chains. Classical-only did:qi roots cannot authorize PQ claims; migration needs an independently trusted checkpoint/rotation ceremony and cannot retroactively prove pre-migration evidence. Registry/DID extensions belong to their parent owners.

QSession uses its reviewed fresh hybrid handshake with both ML-KEM and X25519 contributions, Finished confirmation, separate directional/domain keys, no initial 0-RTT and no downgrade. RFC 10024 is TLS prior art, not automatic QSession certification. SLH-DSA is an optional separately reviewed offline-root profile, not a third mandatory signature on every leaf. PQ data protection and PQ route/discovery metadata claims are reported separately.

Construction order MUST be acyclic:

1. Fix a publication context containing profile, random publication nonce, opaque entry IDs, generation, declared lengths, leaf size, protection class and key epochs. Exclude ciphertext roots and signatures.
2. Hash that canonical context with a dedicated SHA-384 domain to obtain `context_id`.
3. Produce plaintext or encrypted entry representations using that context; compute their trees and metadata-page commitments.
4. Assemble immutable QASD body, compute `descriptor_id`, then generate paired proofs over its binding object.
5. Issue renewable grants/key wraps referring to the completed descriptor/context. They do not redefine the bytes.

Restricted plaintext hashes and names stay in encrypted authorized metadata; the public descriptor must not expose them as confirmation oracles. Opaque object identifiers are not guessed sensitive plaintext digests. Public content may also carry SHA-384 of the complete source bundle; neither a Merkle root nor a plaintext digest is the hash of ciphertext by implication.

## 4. Range proofs and leaves

R1 uses 16,384-byte **entry-relative logical plaintext leaves**. Final leaf length is explicit; empty entries have a defined empty root and no data leaf. A semantic segment references one or more leaves and can span chunks; leaves never span entries. Authenticated header/index and other inventory pages have their own typed commitments. Public raw-file offsets are `entry_base + leaf_index * leaf_size`; encrypted representation offsets are different and must never be passed to a QBDL parser.

Use a SHA-384 binary tree with the largest-power-of-two split rule of [RFC 9162](https://www.rfc-editor.org/rfc/rfc9162.html), no dummy leaf duplication. Domain-separated leaf frames bind context ID, entry ID, representation/key epoch, leaf index, exact logical length, encoded length and bytes. Internal-node and empty-tree domains are distinct. Freeze byte order/field widths in QBS-05. The tree is a QBDL profile, not a claim of standard CT compatibility. Iterative verification has a capped proof depth, no unbounded recursion or allocation. Reject extra/short siblings, wrong final length, reordered leaves and cross-entry/generation splices.

Only one-leaf inclusion proofs are mandatory initially. Batch transport can carry several separately verifiable leaves; efficient multiproofs and inline digest lists are later optimizations. For 16 GiB at 16 KiB leaves there are 1,048,576 leaves and an isolated binary path has 20 × 48 = 960 hash bytes before framing. Do not promise a universal overhead percentage: small/tail leaves and encrypted metadata change it. Full-tree outboard storage is bounded separately and built incrementally on disk.

Partial bytes may reach a codec only after proof/authentication and sufficient metadata/dependencies. Whole-file CRC or whole-entry hash status remains incomplete until all required bytes are present. EMF's 32 KiB record brick plus 64-byte trailer does not fit two 16 KiB leaves; range planning must include the trailer and validate it rather than assuming perfect brick/page alignment.

## 5. Restricted object encryption

QBS-10 must specify and independently review an indexed AEAD construction. Do not label it OAE2/nOAE2-secure solely because it uses ChaCha20-Poly1305. Candidate R1 framing: a fresh 256-bit secret per entry/key epoch, separately domain-derived entry key, a fixed 4-byte nonce prefix followed by an 8-byte big-endian leaf counter, and canonical AAD binding context, entry, generation, epoch, index, logical length and final-leaf status. The key/prefix/counter tuple must never repeat for different plaintext or AAD. A content edit or uncertain crash recovery starts a fresh publication context/key; retransmission uses exact retained ciphertext.

Each leaf expands by the 16-byte tag. Ciphertext frames have a separate representation ID and location map. A fixed-size entry representation can map leaf `i` to `i * (16384 + 16)`, with an exact shorter final frame. Any transport headers are excluded from that mapping and specified separately. No confidential directory is silently inserted into logical QBDL byte zero; it is a distinct encrypted metadata object.

Hash ciphertext-plus-tag into the externally verifiable entry tree. Relays can verify it without keys. After inclusion verification, decrypt into a private lease; on any error quarantine/erase the output, never release partial plaintext. Authorized metadata binds exact plaintext digests/structure. Obtain entry keys through reviewed hybrid recipient envelopes or a confirmed PQ session exporter; no arbitrary export-key shortcut. Refreshing a recipient wrap cannot revoke a key already learned: prospective exclusion needs fresh keys and a new ciphertext generation. Offline copies cannot be recalled.

QBS-03/10 freeze per-key byte/frame/failed-verification limits and aggregate forgery budget. Provisional test ceiling is at most 2^20 frames (16 GiB plaintext) per entry key epoch, reduced by cryptographic review if needed. No production permission derives from that provisional number. Larger encrypted entries require an explicit epoch map or refusal; public u64 objects need not fit RAM.

## 6. Reservations, scheduling and durable storage

| Initial negotiated ceiling | R1 target; admission can choose lower |
|---|---|
| Remote logical bundle extent | 1 TiB, u64; not an allocation request |
| QASD body / complete proof envelope | 64 KiB / 96 KiB |
| Inventory or DAG page | 64 KiB encoded; cumulative metadata budget 8 MiB |
| Entries / dependency nodes / edges | 4096 / 65,536 / 262,144 |
| Proof | 32 siblings, 1536 hash bytes; bounded framing additionally |
| Hot transfer slots | 8 constrained / 64 desktop; higher values require measured host admission |
| Resident transfer window | 8 MiB constrained / 32 MiB desktop, including proof/tag/queue overhead |
| Semantic closure | 16 artifacts, 1 MiB encoded, bounded expansion and existing 42 MiB Sentinel pass |
| Ordinary cell | At most existing 512 MiB ceiling; aggregate host reserve remains separate |
| Legacy HTTP whole-object fallback | At most 1 MiB and explicit policy approval |

These are initial profile choices to benchmark, not changes to global replication constants. Preserve current `MAX_RANGES=8`, 4 MiB decoded control bound, `MAX_BLOCKS=8`, 16 KiB block and crypto-flight ceilings for existing protocols. Add a paged plan/transfer owner. Charge real struct/enum sizes, worker stacks, crypto scratch, persisted buffers and simultaneous queues. Reserve network, verification, decode/storage and consumer capacity before issuing credit. Control traffic retains a non-borrowable reserve.

Use iterative bounded DAG closure and ready queues; fair scheduling across principals prevents one large model from starving control/scientific work. Priority is semantic, not filename substring matching. Representation edges distinguish bit-exact reencoding, calibrated quantization, perceptual proxy and state/ontogenetic alternatives. The latter are never bandwidth substitutions. Stateful inference switches require an admitted clean checkpoint/re-prefill boundary.

Native and OPFS stores track verified ranges separately from sparse file contents: holes are never verified zeros. Write bytes, flush according to the storage contract, then durably commit verified bitmap/checkpoint with descriptor, roots, policy and storage generations. Checkpoints use an authenticated local record or reverify on resume; browser persistence without a trustworthy retained key must reverify. Crash points at every ordering boundary are tested. Reopening after lost writer exclusion requires rehashing relevant bytes. Final promotion is atomic only where the platform promises it; otherwise use a transactional manifest pointer. Pins, custody commitments and evidence holds prevent premature deletion.

## 7. Native service and carrier messages

Use reviewed bounded message variants: `Describe`, `DescriptorPage`, `Select`, `Open`, `Grant`, `RequestLeaves`, `LeafData`, `Credit`, `Cancel`, `Resume`, `Outcome`, `Close`. Each carries profile/schema, descriptor/context identity, request ID, object/entry/leaf range, authority/policy/cancellation generation and appropriate deadline/credit. Actual integer tags belong to QBS-01; no worker invents its own enum. Share/consent occurs before protected disclosure. Maximum message and aggregate reassembly bounds are negotiated before content.

Application credits represent reserved capacity, not transport ACKs. On reliable carriers, reconnect/resume reissues missing verified leaf IDs; it does not run a second congestion/retransmission loop. Duplicate delivery is idempotent and cannot duplicate consumer commit or settlement. Security, framing and policy failures are distinguished internally; public error detail is constrained to avoid disclosure or key oracles.

## 8. HTTP and browser binding

An HTTP-only origin can host an authorized detached descriptor, metadata pages, proof sidecars and immutable public/raw or encrypted representation bytes. Proposed example paths are `/objects/{id}/descriptor`, `/objects/{id}/entries/{entry}/proof/{leaf}`, and `/objects/{id}/representations/{rep}`; these are deployment conventions, not registry allocations. An origin without proofs may support bounded full-entry verification against an authenticated descriptor; it cannot claim authenticated partial delivery. Legacy public downloads remain explicitly non-PQ if the client cannot verify this profile.

For native HTTP clients request a single nonempty inclusive byte range with checked `end=start+length-1`, `Accept-Encoding: identity`, and strong `If-Range` when available. Validate exact 206 Content-Range, pinned total extent, content encoding and actual length. Bound streaming reads before allocating; HTTP framing/transfer coding is decoded by the HTTP stack. Abort ignored-range 200 before consuming a large body; only the explicit small-object fallback is allowed. Treat 416, changed validators and changed representation as refusal/reselection, never cross-generation assembly. Reject multipart in R1. Untrusted Repr-Digest/Content-Digest can assist checks but cannot replace the authenticated descriptor. A representation digest hashes representation bytes, not its Merkle root.

LIG/CSCP reauthorize redirects, changed origins, mirrors and relay paths; never forward credentials across origin changes. Cache by descriptor/root, protection profile and privacy partition. Private plaintext has no shared-cache path. Recovery/cancellation remains object-bound across transport switches. HTTP/1.1 is mandatory compatibility; HTTP/2 and HTTP/3 reuse semantics when supported by the selected client, without making either native dependencies.

Browser JavaScript cannot set forbidden `Accept-Encoding`. The endpoint must serve identity content, expose necessary Content-Range/encoding/validator headers through CORS, and provide verifiable exact bytes; otherwise use an admitted LIG or refuse. Cross-origin manual redirects can be opaque: browsers use a preauthorized stable endpoint or redirect-error policy, not a promise to inspect hidden redirect targets. If `If-Range` or custom headers require preflight, configure explicit allowed headers/origins. Credentials and bearer grants never appear in URLs.

Run storage/hydration in a dedicated worker. Probe OPFS, secure context, storage quota, COI/SAB and WebGPU limits; use a bounded transferable-buffer single-worker path when SAB is unavailable. OPFS denial yields an explicit bounded-memory mode or refusal, not full-object allocation. Device loss, quota changes and tab closure cancel leases safely. Browser JS/browser-internal allocation cannot be called zero-heap; measure Rust/WASM hot paths separately and bound browser queues.

## 9. WebSockets and WebRTC

**WebSockets are R1 (QBS-26).** Use WSS for remote deployments and a negotiated binary application subprotocol; plain WS is only a separately authorized loopback/test context. Carry the same service message variants, proofs, credits and cancellation epochs. TLS termination does not supply PQ publisher authority; restricted objects retain end-to-end PQ key delivery and object encryption. If the browser endpoint itself needs PQ session authority, establish the reviewed QPR application handshake over the reliable carrier before protected control messages. Expose the actual security class, never label ordinary WSS PQ by itself.

One logical record per bounded binary message is the initial application framing. Cap aggregate WebSocket fragmentation, enforce credit before send, disable negotiated compression for the byte-exact/security profile and bound reassembly. Browser WebSocket has no general receive-backpressure API: enforce peer credits, bound event queues and close on violation; this cannot prevent every browser-internal hostile-server allocation, so document the residual limit. Use bufferedAmount to bound sending. Browser constructors cannot set arbitrary Authorization headers; authenticate in a bounded first application exchange, with server Origin checks and CSRF defenses where cookies are used. Origin is not identity. No grants in query strings/subprotocol tokens. Reconnect resumes verified state rather than replaying side effects. Follow [RFC 6455](https://www.rfc-editor.org/rfc/rfc6455.html) and the [browser WebSockets API](https://websockets.spec.whatwg.org/).

**WebRTC data channels are R2 (QBS-27).** Use reliable ordered binary channels first, with the existing reliable-carrier mode, bounded records and request IDs. Raw audio/video/RTP streaming is outside this data profile. DTLS/SCTP security does not replace application PQ authentication or encrypted objects. Bind authenticated signaling offers/answers, peer keys, candidate updates and intended service to a session nonce; prevent signaling substitution and replay. QDNF discovery/signaling is preferred; authorized WSS rendezvous is an optional compatibility route, not a native requirement.

Before ICE gathering, QPolicy/CSCP selects whether host/server-reflexive/relay candidates may be exposed and which STUN/TURN services may be contacted. Restricted profiles can require relay-only; candidate failure never overrides privacy to try direct connectivity. Relays see timing/size even with PQ payload protection. TURN credentials are short-lived and scoped; do not claim a TURN deployment is decentralized or mandatory. Handle offer glare, ICE restart, channel reset, consent loss and changed path authorization explicitly.

Respect negotiated `maxMessageSize`; fragment application records if necessary under a bounded reassembly lease. Use bufferedAmount/low-water signals and credits. Separate proof/control priority from bulk data without assuming independent congestion domains. Start with one reliable ordered mode; unordered/retransmit-limited channels need separate loss/replay/completion proofs. Test direct and relay paths in isolated fixtures. Follow [RFC 8831](https://www.rfc-editor.org/rfc/rfc8831.html) and [WebRTC](https://www.w3.org/TR/webrtc/). All three compatibility bindings share the same authenticated test vectors and cache identity.

## 10. Format, Etalon and Commons integration

10D uses its actual 24-byte descriptor table. Partial admission checks authenticated header/table and complete selected sections/dependencies; whole-file CRC status stays incomplete. EMF/M64 adapters include repaired full trailers, row/header checks and selected complete chunk semantics. Q42 uses its existing volume/range owner, with policy/semantic closure. Do not inflate compact Quin identifiers or introduce a second codec implementation in the transport.

P64 R1 downloads a verified complete entry into an excluded-writer snapshot or bounded extracted file before calling the current whole-file view/preparation path. Reserve RAM/storage/VRAM first; no sparse holes masquerading as tensors. Page alignment is tested at actual mapping addresses. Selective operator-package/P64 tensor execution is QBS-21 and needs validated range APIs, metadata/ETLIMD1/ETLMOE2, dtype/alternate-view rules, dependencies, KV lifetime and consumer commit. Existing APIs must be inspected, not recreated from names in the brief.

Compile signed semantic contracts into scoped grants; verify current legal licence and publication permissions separately from creation-cost fulfilment. Reuse personal/protected/humanitarian exemptions. Paid corporate admission uses existing finite obligation owner and durable idempotent operation IDs; retries do not create a new charge. Billing basis (verified bytes, accepted consumption, etc.) is explicit in the agreement, not inferred from packets. Use checked integer units/carry so splitting a transfer cannot change the total. Delivery, storage custody, compute acceptance and financial settlement are separate receipts. RFC 9942 inclusion evidence is not itself financial settlement.

For distributed escrow, do not reclaim rights solely on TTL expiry while an isolated holder can still spend them. Require acknowledged return, enforced fencing, or conservatively retain outstanding rights until reconciliation. Maintain `settled + authorized_discharge + all_outstanding_rights <= target`, counting grants/holds once. Model crashes, partitions, duplicate receipts, clock changes and grant transfer before enabling fee collection. Public/exempt paths never wait for this optional recovery-fee machinery.

Solver Commons starts with canonical exact problem serialization and bounded local witness verification. A miss is scoped search failure, not mathematical novelty. Approximate normalization/isomorphism produces a candidate requiring equivalence/error evidence, never an exact-hit key by assertion. Keep variable rehydration maps private, assess topology disclosure, distinguish feasible/heuristic/bounded/certified results, and obtain explicit cost/disclosure admission before QPU dispatch. Generation streams pin predecessor and semantic state; arbitrary updates cannot use append-consistency proofs unless their committed history is actually append-only.
