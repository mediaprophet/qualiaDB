# QPR Semantic Peer Services

**Status:** Proposed service profiles 0.1; schema and wire-vector freeze required before interoperability

These services make [Qualia Peer Runtime](./peer-runtime.md) useful beyond secure byte streams.
They reuse QSession, QPolicy, QSync, existing QualiaDB modalities, and Q42 storage. All capacities
below are draft maximums that remain subordinate to the runtime's aggregate memory/work limits.

The [PQ replacement profile](./post-quantum-security.md) additionally requires dual authority proofs
and typed SHA-384 commitments. SHA-256 operation/tree recipes below describe the initial classical
record profile. The PQ schema uses separate domain/version labels and 48-byte digests recomputed
from exact source bytes throughout; negotiated record profiles must never be mixed or truncated.
Classical custody encryption likewise cannot satisfy a PQ custody claim.

## 1. Service descriptors and contracts

A signed descriptor carries the following bounded references. Negotiation binds the exact descriptor
digest, selected version, and critical features to `SERVICE_OPEN`.

| Field | Purpose |
|---|---|
| Service identity and provider authority | Full service IRI, version, resource/graph identity, provider grant, issue/expiry and sequence |
| Operation set | Explicit publish, subscribe, read, append, mutate, sync, custody, or RPC operations; no inference from connectivity |
| Data contract | Message schemas and media types; exact ontology/context/compression/shapes/rules bundle for semantic channels |
| Execution profile | Bounded validator/evaluator version, supported filters/merge semantics, maximum record/expansion/work limits |
| Delivery profile | Stream/datagram mode, ordering, expiry, replay/retention scope, gap behavior, and receipt meanings |
| Governance profile | Audience, purpose, context, sensitivity, consent, policy generation and revocation requirements |
| Resource profile | Byte/time/energy scope, provider limits, accepted funding/quote reference where needed |

An opaque byte service may use bounded structural schemas without an ontology. A channel claiming
ontology-defined contracts MUST negotiate [the pinned CBOR-LD contract profile](./ontological-contracts.md).
The contract describes rights, duties, quantities, and interpretation; the runtime compiles supported
terms into bounded decision/validation handles. Unknown duties or unsupported evaluators cannot
become allow decisions. Description Logic or a model-generated interpretation does not automatically
make a contract executable.

Keep exact signed CBOR-LD bytes with their semantic-bundle digest in Q42. Quin projections accelerate
selection and policy checks without becoming substitute signature material. Message schemas specify
which fields are required, critical, and signed. On-wire schema IDs and all expansion/decoder limits
must be fixed by the profile; the current permissive codec fallbacks do not meet this requirement.

## 2. Semantic subscriptions

`qdnf:service:pubsub` offers two explicit modes:

- **Event feed:** bounded notifications with stable event identities and declared retention. Without
  retained history, missed events produce a gap; successful transport is not durable delivery.
- **Graph projection:** a snapshot of an authorized view followed by additions/removals relative to
  that snapshot. A retention gap or invalid checkpoint requires an explicit new snapshot.

Subscription admission checks both the right to inspect the source and to receive the selected
projection. The request binds scope, subscriber persona, service/contract, compiled filter digest,
selected fields, delivery mode, maximum output bytes/rate, expiry, and optional agreement. A service
may require consent before revealing that the underlying graph or subscription exists.

The first graph-filter profile supports a bounded positive conjunction of at most eight triple
patterns in one authorized graph, with explicit projected fields and typed literal comparisons.
Joins, candidate scans, result rows, and retained join state have hard budgets. Full IRIs/datatype
semantics must survive hashing; matching only a truncated hash is insufficient. Unbounded paths,
arbitrary remote SPARQL, unrestricted N3 execution, cross-context joins, and negation over incomplete
data are unsupported in this profile. More expressive evaluators need distinct negotiated profiles.

Compilation binds the filter, semantic bundle, evaluator version, and visibility policy. The kernel
matches candidate deltas using compact Quin indexes; cold work handles bounded joins/re-evaluation.
Evaluate derived facts only under the admitted reasoning profile, with provenance and uncertainty.
Contradictory assertions may be quarantined using core modalities; they do not imply an unrelated
permission, false deletion, or an arbitrary new value.

### 2.1 Snapshot and live handoff

1. Authorize the view and atomically pin a Q42 snapshot generation plus a committed-delta journal
   cursor. The snapshot/journal cut must refer to one core commit boundary.
2. Emit snapshot pages within receiver credit. Each page binds the authorized view and snapshot
   digest; no page may include an inaccessible predicate, object, provenance field, or graph count.
3. Retain/pin deltas after that cut while the snapshot is read. If journal retention cannot cover
   the handoff, stop with `ResnapshotRequired`; never pretend that live delivery is complete.
4. Emit bounded delta batches with stable event IDs and the source commit/view generations.
   Removal from a view is distinct from deletion of the underlying fact.
5. For a recoverable projection, advance the resume cursor only after the subscriber atomically
   commits the applied projection state and checkpoint/cursor. Transport ACK and buffer consumption
   do not establish that boundary. New sessions reauthorize the view before using a token; it is
   a cursor, not a reusable read capability. Lost or non-durable local projection state requires
   resnapshot rather than resuming beyond unapplied deltas.

A resume token binds subscriber persona, scope, filter/contract digest, visibility-policy epoch,
snapshot/checkpoint digest, last acknowledged event, retention epoch and expiry, authenticated by
the issuing service. A global positional offset in a mutable relay vector is insufficient.
Changing membership or visibility invalidates affected tokens and previously queued disclosure.
Reconnection cannot revoke facts the subscriber has already learned.

### 2.2 Bounded dissemination

Distribute admitted event references over a small authorized mesh; fetch large bodies through
QSync/content transfer. Draft general-node limits are 64 active subscriptions, target degree six,
maximum degree eight per subscription, and a maximum of 32 digest references per advertisement.
These are admission caps, not a mandate to allocate 512 peer connections: subscriptions reuse
authorized sessions and share aggregate state. Small/private groups may use pairwise fanout.

Initial mesh selection reserves up to two locally initiated links where authorized candidates
permit it, rotates candidates under bounded churn limits, and considers independent available
routes. Local protocol-behavior scores and backoff can resist abuse; they cannot prove Sybil
independence or guarantee delivery through a partition. No access right follows from a score.

Within an authorized mesh, a session-bound interest message advertises a scoped, expiring topic
token and supported profile. Digest advertisements, bounded fetch requests, accepted event messages,
and unsubscribe/backoff messages form the initial vocabulary. They are application service messages,
not newly assigned QFrame types. Before enabling interoperability, freeze their CBOR/CDDL forms,
state transitions, retry limits, dedup windows, and exact signature/encryption coverage.

Public topics may reveal their meaning deliberately. Private topic tokens derive from a scoped
secret and membership epoch; plaintext queries and stable global topic hashes are not broadcast.
Relays without semantic read authority forward encrypted bodies using scoped routing tokens only.
Opaque-token equality, timing and traffic volume can still reveal relationships; this is not an
anonymity guarantee. Group-wide keys require the separate group-key profile; pairwise encryption
is the baseline for membership changes.

Each hop enforces publisher/relay/subscriber grants, message length/expiry, bounded duplicate state,
credit and fanout before forwarding. Origin identity, scope, event ID, payload digest, contract and
expiry are authenticated end to end. Relay permission does not grant permission to decrypt or
republish in a different scope. Validation failures have scoped penalties; withheld optional data
or unreachable peers are not automatically misconduct.

A digest hint or Bloom hit never proves that all required events arrived. A graph projection uses
periodic authorized checkpoint comparison and QSync recovery. An event feed without that recovery
must expose its weaker delivery behavior to the application.

## 3. QSync anti-entropy and operation semantics

The resumable-sync profile extends `qdnf:service:sync`. It synchronizes a specifically authorized
dataset/view, not the entire underlying Q42 file by default. A raw file transfer is allowed only
when the requester is authorized for every byte and all disclosed metadata in that artifact.

### 3.1 Operation identity and validity

Each signed operation envelope binds:

- full dataset/scope and contract/merge-profile digests;
- author/controller proof, author epoch and monotonic sequence;
- operation ID, causal parent references, and operation kind;
- exact payload digest, payload encoding, and bounded length;
- purpose/audience and the applicable authorization evidence; and
- issue/expiry or explicit historical-admission semantics.

The proposed operation ID is SHA-256 of deterministic CBOR encoding of the array
`["qdnf:sync:op-id:v1", scope_digest, author_controller_digest, author_epoch, sequence]`.
Fixed field types and digest algorithms must be frozen in vectors. The full signed-envelope digest
is separate. Two different envelope bodies under one operation ID establish author equivocation
only after both signatures and their author authority verify. Preliminary duplicate/conflict lookup
may precede expensive verification, but an invalid candidate consumes transient resources only: it
cannot quarantine accepted state or penalize the claimed author. Valid conflicts enter a bounded
quarantine path; do not let last arrival choose the effect. Identical authenticated retry bytes
return the existing admission/result record without executing the operation again, only to a caller
currently authorized to receive that outcome.

Sequences never wrap. A new author epoch is explicitly authorized and links to its predecessor or
an approved initial checkpoint. Reconnect, key rotation, compaction and software rollback cannot
silently reset replay identity. At most 16 causal parents per operation and 256 pending dependency
entries per admitted sync scope are initial caps. Excess authors, dependencies, missing history or
oversized operations require a negotiated checkpoint/profile or rejection, not truncated validation.

### 3.2 Authenticated set comparison

Q42 BIDX/PIDX and compact postings prune local candidates; their short hashes/Bloom filters are not
network proofs. Construct an authorized checkpoint manifest over a canonical ordered set of full
operation IDs and envelope digests, plus its scope, contract, merge version, generation, and causal
frontier. Sign its root with an authorized checkpoint issuer. A signature attests the issuer's
checkpoint, not that the issuer is honest or that all possible operations are included.

For the initial profile, use SHA-256 with distinct leaf and internal-node domains. A leaf hashes
deterministic CBOR `["qdnf:sync:leaf:v1", scope_digest, operation_id, envelope_digest]`; an internal
node hashes `["qdnf:sync:node:v1", left_digest, right_digest]`. Sort leaves lexicographically by the
full operation ID. Duplicate IDs are invalid; the empty root has its own domain. Odd nodes promote
unchanged to the next level. The manifest binds leaf count and tree version. Exact empty-root bytes,
tree shapes, count checks and proof vectors are freeze requirements, not implied by this prose.

Peers exchange checkpoint roots, then bounded subtree/page differences and missing exact objects.
Draft limits: at most 256 leaf references per response page, at most 64 sibling hashes per proof,
and at most 64 KiB encoded control data per service message. Smaller negotiated limits always win.
Maintain a bounded iterative traversal stack and return continuation state when output fills.
Receiver verifies object digest, origin signature, authority and merge semantics even when a
checkpoint proof succeeds. Range completeness needs verified subtree counts/bounds; membership
proofs alone do not show that a requested range has no omissions.

Proofs and roots apply to the authorized dataset partition/view, constructed without hidden global
neighbors or private counts. Do not disclose a whole-database Merkle proof to a subscriber who may
read only one projection. A provider unable to construct the permitted view within budget returns
an explicit limit/unsupported outcome. Authority-filtered snapshots may need new materialized Q42
segments; they are bounded core work, not a network-maintained duplicate database.

Filtered recovery MUST choose a provenance mode that the requester is authorized to receive:

- **Complete source operations:** authorization partitions align with complete signed envelopes,
  including their payload, parents and provenance. Transfer preserves the original signature.
- **Projector-attested operations:** an explicitly authorized projector emits new signed view
  operations with their own identities, projection/filter and policy version, a scoped checkpoint,
  and disclosure-safe provenance. They attest the projector's derived view, not the source author's
  signature over the redacted bytes. Recipients must accept this distinct authority/merge contract.

Redacting fields or causal references from a signed source operation invalidates its signature.
Materializing a filtered Q42 segment alone does not solve that problem. If neither mode is available,
deny the requested view or report unsupported recovery; never disclose hidden data to complete a
proof. Projector authority is separately granted by `qdnf:op:derive-projection` and cannot be inferred
from read, subscribe or relay permission.

### 3.3 Admission, merge and recovery

On receive, check version, lengths, scope, replay/conflict identity, signature/controller authority,
capability, sensitivity, semantic validity and dependencies before making data visible. Persist
accepted identity, resulting graph changes and result receipt in one atomic/recoverable core commit.
After a crash, either no effect exists or recovery returns the committed effect and its receipt.
Do not acknowledge durable application merely because bytes reached a relay or WAL append returned.

Wire delivery is at least once where retries/retention are enabled. At-most-once local application
requires durable deduplication and an atomic application commit for the declared retention/epoch.
It is not a general exactly-once guarantee across partitions or external systems. Retry safety must
remain explicit for RPC actions that cannot share the core transaction.

Each dataset declares its merge semantics. Existing LWW comparison is appropriate only where the
application accepts its loss of concurrent values; multi-value/conflict-sensitive data retains
alternatives. Membership changes and revocations use authority rules, not an arbitrary LWW winner.
Physical timestamp order must not be substituted for causal order.

Deletion produces an authenticated tombstone with causal context. Compact tombstones/replay state
only after an authorized checkpoint and the profile's acknowledgement frontier cover all required
replicas, or an explicit epoch transition excludes stale replicas. An excluded returning replica
must resnapshot/re-authorize; it cannot resurrect deleted records by presenting an old operation.
Timeout alone does not prove causal stability. Expiry of a presence event and deletion of a durable
fact are different operations.

## 4. Content transfer

Use the existing content/swarm service for immutable Q42 segments and exact signed artifacts.
A manifest binds full artifact and block digests, byte ranges, codec/version, maximum decoded size,
and provider authorization. A provider may prove possession of bytes without having authority to
interpret or disclose them. Ciphertext-addressed content and plaintext-addressed content have
different equality/privacy implications; the descriptor selects one explicitly.

Schedule independent blocks across up to three validated paths under aggregate receive/storage
credit. Verify each admitted block and final artifact before promotion. Partial blocks, decompression
scratch and old/new generations remain charged; digest failures cannot force unbounded retries.
Resume refers to verified blocks in a pinned manifest, not an unchecked byte offset into a changed
file. Optional erasure coding, network coding or GPU verification need separately bounded profiles
and measured benefit; they are not prerequisites for the initial runtime.

## 5. Encrypted custody for intermittent peers

`qdnf:service:custody` allows a sender to deposit an encrypted envelope for later retrieval by an
authorized recipient. A custodian grants bounded bytes, object count, retention, permitted transfer
audience, and optional compensation. Live forwarding and durable custody are separate capabilities.

The outer envelope binds a scoped mailbox token, unique custody/object ID, ciphertext digest/length,
expiry, sender authorization, allowed custodian/recipient roles, and optional agreement. The inner
authenticated message binds the intended recipient persona, original operation identity and service
contract. Session keys are not stored for later replay. Offline encryption requires an authorized,
expiring recipient encryption-key advertisement and a frozen envelope/key-rotation profile using
the cryptographic profile's primitives.

Custody proceeds through distinct receipts:

1. **Offered:** advertised capacity and terms; no storage guarantee yet.
2. **Stored:** verified envelope durably committed under the custody grant, with retention deadline.
3. **Retrieved:** authenticated recipient obtained the bytes; payload processing may still fail.
4. **Applied/accepted:** recipient independently signs the service outcome, where supported.
5. **Released/expired:** custodian stops serving and reclaims storage under the contract.

Retries reuse the same custody ID and ciphertext digest. Mismatched content under an ID is a
conflict. Bounded multi-custodian replication binds the replica set/maximum and budget; each recipient
deduplicates by the inner operation ID. No custodian can claim successful application or collect
application-dependent compensation from a storage receipt alone.

Custody never extends an operation's permission or execution deadline. If delayed work needs
historical authorization, the service must explicitly define and validate it; otherwise obtain a
fresh grant. Membership/key rotation can make a stored object undecryptable. Static offline
encryption does not automatically provide forward secrecy after recipient-key compromise; stronger
prekey/ratchet and removal semantics require a separate reviewed profile. Deleting local bytes
cannot prove that every recipient or malicious custodian erased a prior copy.

## 6. Governed compute and RPC

Use QDP/qapp RPC for remote compute rather than adding a second job protocol. A request binds
service/executable version, capability manifest, input artifact digests, permitted outputs and side
effects, deadline, cancellation, resource reservations, and accepted contract where applicable.
Only locally installed/admitted executors may run; a semantic description is not execution authority.

Reuse bounded Webizen evaluation and existing compute services where their actual interfaces fit.
Separate deterministic tasks that can be replayed from nondeterministic device/model work. A result
receipt records executor and input versions, result digest, verification method, and measured or
estimated energy/time. A signed receipt is an accountable claim, not cryptographic proof of correct
computation. Re-execution, sampling, domain validators, or a separately specified proof system may
provide additional evidence.

No mandatory GPU, LLM, token economy or global marketplace is introduced. Scheduling can place work
near permitted Q42 data, reduce transfers, or use an explicitly authorized remote provider. The
planner applies privacy and execution constraints before cost preference. Spending authority and
result-acceptance authority remain separate from permission to invoke a tool.

## 7. Existing primitives and missing guarantees

| Source anchor | Reusable capability | Work required before claiming this profile |
|---|---|---|
| [daemon graph revisions](../../../../crates/qualia-core-db/src/services/daemon_graph.rs), [pulse transport](../../../../crates/qualia-core-db/src/services/pulse_transport.rs) | Mutation notifications and topic publication | Bounded durable delta journal, view projection, gap/resnapshot and snapshot/live atomic cut; string broadcasts are not semantic deltas |
| [graph index](../../../../crates/qualia-core-db/src/query/graph_index.rs) | Revision-aware local index construction | Bounded incremental maintenance; current snapshot rebuilding is not a network subscription engine |
| [sync node](../../../../crates/qualia-core-db/src/p2p/sync_node.rs), [sync relay](../../../../crates/qualia-core-db/src/p2p/sync_ops.rs) | Working signed-operation byte transport | Replace unbounded commands, whole-suffix clones, in-memory vectors and oversized frames with scoped Q42 pages and aggregate limits |
| [inbox](../../../../crates/qualia-client-core/src/wellfair/sync_protocol.rs), [outbox](../../../../crates/qualia-client-core/src/wellfair/sync_outbox.rs), [sync API](../../../../crates/qualia-client-core/src/wellfair/api/sync.rs) | Durable append/replay patterns | Real full-envelope signature verification, bounded recovery and atomic dedup/application; presence-only signatures and read-check-append races are inadequate |
| [structural diff](../../../../crates/qualia-core-db/src/sync.rs), [CRDT resolver](../../../../crates/qualia-core-db/src/foundation/crdt.rs) | Caller-buffered comparison and deterministic local conflict resolution | Strong authenticated digests, continuation/error on full output, canonical clocks, causal deletion; current names do not establish Merkle-proof guarantees |
| [geometry workspace](../../../../crates/qualia-core-db/src/specialized_libs/computational_geometry/geometry_workspace.rs), [coordination](../../../../crates/qualia-core-db/src/governance/coordination.rs) | Budget/cancellation and resource-declaration patterns | Audit alignment/arithmetic; implement networking leases and aggregate reservations rather than copying assumptions |
| [platform scheduler](../../../../crates/qualia-core-db/src/platform/platform_scheduler.rs), [local scheduler](../../../../crates/qualia-core-db/src/platform/local_scheduler.rs) | Platform QoS and bounded mailbox patterns | Actual network execution, fairness and ownership enforcement; simulated work is not a production peer executor |
| [thermal telemetry](../../../../crates/qualia-core-db/src/inference/thermal_telemetry.rs) | Real sensor inputs on supported hardware | Preserve unavailable readings and integrate/attribute energy over time; instantaneous watts are not per-operation joules |

These anchors were inspected for the design on 2026-09-06. They identify code reuse, not completed
QPR modules. The [broader source review](./source-and-current-stack-review.md) records the existing
crypto, contract-codec, core-storage and payment boundaries as well.

## 8. Interoperability and acceptance

Optional features are registered in [Registries and Extensibility](./registries-and-extensibility.md).
Until schemas, exact-byte vectors, and independent receiver tests exist, these remain draft profiles.
The implementation must cover:

- Subscription snapshot/live races, disconnected consumers, journal overflow, joins exceeding work
  limits, scope changes, private-topic discovery and hidden-field/count disclosure.
- Mesh churn, duplicate floods, withheld events, slow receivers, bounded fanout and failed recovery;
  compare a repaired authorized checkpoint before declaring a graph projection current.
- Operation-ID conflicts, wrong controller/contract, delayed dependencies, concurrent authors,
  restart/rekey, full output buffers, and cryptographic collision-boundary checks.
- Incorrect Merkle proofs, omitted ranges, private partition leakage, malicious checkpoints,
  stale replicas and tombstone compaction; convergence only under the declared merge assumptions.
- Crash at each custody and inbox/application/receipt boundary, wrong recipient, duplicate retrieval,
  expiry, lost keys, reservation exhaustion and compensation based on the wrong receipt stage.
- Compute cancellation, dishonest result claims, duplicate external effects, unknown telemetry and
  cross-provider spending caps, with no permission widened by successful payment.
