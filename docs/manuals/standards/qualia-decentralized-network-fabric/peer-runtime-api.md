# QPR Runtime Model and API

**Status:** Proposed API and execution contract 0.1; sketches are not compiled Rust interfaces

This document defines the implementation discipline for [Qualia Peer Runtime](./peer-runtime.md).
Wire formats remain owned by QDNF. The public facade must make authority, memory ownership,
cancellation, and completion visible enough that applications can use them correctly.

## 1. Two execution tiers

| Tier | Work | Contract |
|---|---|---|
| Cold bounded construction | Load/verify pinned semantic bundles; compile service/policy plans; construct route/index generations; configure host adapters | Caller-budgeted workspace, cancellation, output limits, immutable published result; bounded allocation where repository rules permit it |
| Hot execution | Parse fixed headers; check generations; reserve counters; advance protocol states; schedule frames; filter admitted records; produce events/effects | No heap allocation on success, denial, timeout, or malformed-input paths; no unbounded recursion; fixed/caller-owned storage |

Expensive work is not automatically cold just because it is infrequent. Every remote trigger must
first acquire a transient work quota. Host workers receive a byte limit, instruction/work limit,
deadline, and bounded output slot. A hostile stream of unique contexts must not create unlimited
background ontology loads or signature tasks. Cache misses return a bounded pending/non-allow state.

The hot kernel is a state machine without direct I/O. Given the same initial generations, admitted
event ordering, clock samples, configuration, and cryptographic random inputs, it produces the same
state transitions and effects. Network scheduling is not globally deterministic. Replay fixtures
inject recorded random inputs; production challenges/keys always use fresh cryptographic entropy.
Production diagnostics MUST NOT persist secret random material to make traces reproducible.

## 2. Host and kernel boundary

The host owns sockets/bearers, storage access, key custody, worker execution, and scheduling. The
kernel owns protocol state and admission decisions. Hosts must not treat an application-supplied
number as a verified capability handle.

```text
host ingress -> owned input lease -> cheap bounds/replay checks -> admitted kernel event
kernel step -> bounded effect slots -> host I/O or cold job -> completion event
validated service data -> bounded application event -> consumer completion -> lease release
```

Every asynchronous effect has a runtime instance ID, operation ID, slot generation, byte/work
reservation, and cancellation epoch. A completion for a recycled slot is rejected. A completion
after cancellation releases outstanding storage but cannot resurrect the operation. In-flight
kernel/OS access keeps a lease pinned until the host acknowledges completion or definitive abort.
Cancellation alone does not make its memory reusable.

Monotonic time drives scheduling, deadlines, RTT, and elapsed measurement. Wall-clock authority and
its uncertainty drive signed expiry checks. Clock rollback cannot extend previously accepted
authority; after restart, reconstruct remaining validity conservatively or revalidate it. Logical
CRDT clocks are a third, separate domain and do not measure physical seconds.

## 3. Handle and buffer model

Handles are opaque local references containing an instance identifier, table slot, and nonwrapping
generation. They are not wire identities, CPU pointers, authorizations by themselves, or durable
references. On generation exhaustion retire the slot/instance; do not wrap into an old valid handle.
The core stores full identifiers/digests behind verified handles using the canonical NQuin layout.

The proposed [Q42 networking modality](../q42-network-modality-draft.md) distinguishes source
evidence, semantic projections and compiled views. The Quin itself has five non-parity u64 fields
and one parity u64; 60-bit dictionary payloads are only local indexes. Full PQ digests/proofs live
in exact evidence records, with typed references, rather than being truncated into a Quin field.

| Handle | Invariants |
|---|---|
| `TargetHandle` | Verified persistent target and permitted resolution scope |
| `ServiceHandle` | Full IRI/version and descriptor digest, supported profile, current policy generation |
| `AgreementHandle` | Accepted exact contract/bundle digests and current duty state, when required |
| `ReservationHandle` | Explicit parent scopes, capacity/exposure limits, owner, expiry, remaining amounts |
| `OperationHandle` | Request identity, cancellation epoch, deadline, lifecycle, outstanding leases |
| `BufferLease` | Pool slot/generation, initialized length, capacity, access mode, owning operation |
| `SnapshotHandle` | Q42 generation, authorized view, pinned blocks/ranges, bounded lifetime |

The API has two buffer paths. Small control messages are copied into pre-reserved slots. Large
payloads transfer a lease; they are not silently copied into a growing queue. A borrowed input slice
may be parsed during a call but must not escape it. A submitted lease is consumed only if admission
succeeds; on failure the caller receives the same lease back. Leases are not `Copy` or forgeable
public structs. A browser/FFI handle table enforces equivalent single-owner semantics.

Read-only ranges may be shared through bounded reference counts. Decryption and encoding require
exclusive writable leases. No kernel or application holds a raw mmap pointer after its snapshot
pin ends. Zeroize secret-bearing buffers before another authority scope can reuse them; raw
cryptographic session keys stay outside durable Q42 records.

## 4. Facade sketch

The following names express the intended shape. Supporting types are deliberately abstract; this
is not a second ABI definition or a claim that these methods already exist.

```rust
// Cold: buffers, tables, services and host capabilities have explicit ceilings.
fn prepare_into<'a>(
    config: &PeerConfig,
    core: VerifiedCoreView<'a>,
    storage: PeerStorage<'a>,
    scratch: &mut ColdWorkspace,
) -> Result<PeerKernel<'a>, PrepareError>;

// Hot: no direct network/disk I/O, allocation, or reentrant callbacks.
impl PeerKernel<'_> {
    fn submit(&mut self, request: Request)
        -> Result<OperationHandle, RejectedRequest>;

    fn ingest(&mut self, input: Input)
        -> Result<(), RejectedInput>;

    fn step(
        &mut self,
        now: ClockSample,
        budget: StepBudget,
        effects: &mut [EffectSlot],
        events: &mut [EventSlot],
    ) -> StepResult;

    fn cancel(&mut self, operation: OperationHandle) -> CancelState;
    fn complete_event(&mut self, event: EventHandle) -> Result<(), PeerError>;
}

// Illustrative service request: all references are locally verified handles.
struct OpenService {
    target: TargetHandle,
    service: ServiceHandle,
    agreement: Option<AgreementHandle>,
    reservation: ReservationHandle,
    deadline: MonotonicDeadline,
}
```

`PeerStorage` borrows all configured kernel tables and arena partitions. `VerifiedCoreView` is a
bounded view; construction must not load the whole database. `Request`/`Input` consume optional
leases; `RejectedRequest`/`RejectedInput` return ownership plus a fixed error code. Submission
reserves command, completion, and cleanup capacity atomically before changing state.

`StepBudget` limits admitted input count, protocol transitions, bytes processed, verification work,
and output count. A time limit alone cannot bound a nonpreemptible primitive; individual work units
must also have fixed maximum input and execution bounds. `StepResult` reports written slot counts,
work consumed, the next deadline, and whether progress needs input, output space, or another quantum.
It never treats a full output slice as successful delivery of the remaining results.

Effects include transmit, timer update, bounded core read/commit request, verification/compile
request, key operation, and cancellation. Events include service opened/denied, data available,
projection gap, operation accepted/applied, resource pressure, and terminal outcome. Each output
slot has explicit initialization/consumption state; the host must acknowledge it before reuse.

`complete_event` releases a data event's lease and associated receive credit. It is a local
consumption acknowledgement, not evidence that a remote operation was durably applied. A separate
core commit completion creates that evidence. If the application stops consuming, service flow
control stops granting credit; bounded timeout may close the service.

An ergonomic async wrapper may await these events. It uses bounded channels and a shared readiness
loop; no unbounded command channel, hidden runtime, or task-per-packet fanout. C/WASM facades use
fixed integer handles and caller-provided memory, with explicit cancel/release functions and
validation of every range. ABI extraction and target builds are release gates, not current facts.

## 5. Aggregate resource accounting

The Sentinel ceiling is **42 × 1024 × 1024 bytes per execution pass**, not 42 MiB for each protocol
component. One admitted pass accounts for its live kernel state, referenced working pages, retained
input/output, verification/decode scratch, and concurrent worker subleases. A single host-wide
ledger also bounds simultaneous passes, I/O queues, caches, persistent storage, and kernel socket
buffers where controllable. Host memory outside a pass must be reported, never hidden as zero.

Reservations belong to a bounded graph of scopes: host, runtime, transient work, context/service,
peer/session, and operation/agreement. Charge one physical allocation once at each distinct
applicable aggregate scope; two paths to the same ancestor cannot double-count it. Admission either
reserves all affected counters or changes none. A single-owner admission loop is the initial design;
parallel workers receive carved-out leases and do not race independent global counters.

This illustrative 42 MiB configuration is an acceptance-test budget, not a measured capacity claim:

| Partition | MiB | Includes |
|---|---:|---|
| Kernel tables | 4 | Handles, timers, scheduler, session/stream descriptors, small control queues |
| Routing and evidence working set | 6 | Active bounded route generations, verified record projections, pinned lookup pages |
| Receive/reassembly pool | 8 | Admitted wire data, fragments, application data still awaiting consumption |
| Send/retransmit pool | 8 | Owned outbound data retained until applicable transport completion |
| Decode, crypto and policy workspace | 6 | Concurrent worker subleases and contract compilation scratch |
| Q42 query/delta working pages | 6 | Range windows, proof verification, subscription outputs and index work |
| Reserved control and reclamation | 2 | Closure, revocation, essential ACK, completion and cleanup progress |
| Alignment and conservative margin | 2 | Slab padding, bounded stacks and audited runtime overhead |
| **Total** | **42** | **44,040,192 bytes** |

Concrete table sizes MUST fit their partitions using checked arithmetic, alignment, and worst-case
record sizes. The configuration is rejected if they do not. A low-memory profile reduces admitted
neighbors/streams/subscriptions under the negotiated protocol limits. A 1 MiB receive-window ceiling
per connection is not permission to advertise 1 MiB credit on every connection: issued credit must
be backed by aggregate receive capacity. Lowering local memory after credit is issued cannot revoke
the peer's already authorized sends; drain existing commitments first.

Bytes already committed to reassembly, retransmission, pinned Q42 generations, asynchronous effects,
or undelivered application events remain charged. Pools may lend unused pages across classes only
when essential reserves and all outstanding commitments remain satisfiable. Data pressure cannot
evict replay protection, unresolved settlement intent, or tombstones to create apparent capacity.

## 6. Scheduling and energy/time

Use bounded deficit round-robin across admitted context/service scopes, then peers/operations,
with deterministic tie-breaking. Separate capped queues cover essential control, interactive
service work, and background replication/compute. Configure minimum progress and maximum shares;
strict priority alone could starve background reconciliation, while an unlimited control queue
would let attackers monopolize the host. Even essential control is authenticated/rate-limited
where possible and constrained by an independent ingress/transient budget.

Unknown senders are charged to a global transient pool before expensive verification. Per-peer
quotas do not defeat Sybil identities; admission policy, realm limits, and diverse authorized routes
remain necessary. Reputation records describe observed protocol behavior, with scope and expiry.
They are not global human scores and cannot override consent or privilege wealthy participants.

Every admitted operation has a deadline, step quota, byte budget, and configured resource dimensions.
Meter energy as an integral of power over a declared monotonic interval with attribution method and
uncertainty. Scheduler estimates may use calibrated bytes/work counters, but preserve their status
as estimates. GPU/CPU temperatures and unavailable sensors have separate availability states.
Missing telemetry must not silently become zero joules or a known-cool device.

Thermal pressure can reduce concurrency or defer optional work. Explicit contact windows can batch
background synchronization. Neither changes signed route metrics packet by packet or suspends
agreed duties without an outcome/receipt. Monetary and contribution exposure is reserved across
all concurrent operations, including unreported work and unresolved adapter submissions, following
[Commons and Resource Economics](./commons-and-resource-economics.md).

## 7. Q42 publication and policy changes

Durable source objects and projections use [Core Storage and Cache](./core-storage-and-cache.md).
The runtime never reconstructs a valid signature from a Quin-only representation or stores exact
protocol bytes in an unrelated model-weight format. Index-only records point to their exact source.

Build replacement route/policy/subscription generations outside the hot pass. Verify them, charge
their simultaneous old/new memory, and publish at a bounded scheduling boundary. Already queued
application deliveries and asynchronous commit requests carry the generation under which they were
admitted; before delivery/commit, check current authorization or cancel/re-evaluate them. Durable
commit and policy revocation need a shared ordering boundary, so a worker cannot commit stale
authority after the revocation has taken effect.

Retirement waits for all read leases and external effects using the old generation. Bound this
grace period and stop new admission if a stuck consumer prevents reclamation. Do not free memory
still referenced by a worker. Disk compaction has the equivalent pinned-generation rule.

Operation acceptance, replay identity, application changes, and durable result receipt require an
atomic core publication protocol or recoverable commit marker that binds them together. Existing
WAL/manifest helpers are primitives, not evidence that this transaction already exists. Recovery
must use bounded scans/checkpoints and return ambiguous effects for reconciliation. An RPC against
an external system cannot inherit atomicity from a local Q42 commit.

## 8. Failures and observability

| Outcome | Caller behavior |
|---|---|
| `WouldBlock` | Keep returned ownership; retry only when capacity/input changes or a bounded timer fires |
| `ResourceLimit` | Operation was not admitted; choose a smaller request or explicitly revised limits |
| `PendingEvidence` | Wait within a deadline for bounded verification; no application authority yet |
| `Denied` / `UnsupportedProfile` | Do not substitute another service, semantics, or payment path |
| `StaleHandle` / `ResnapshotRequired` | Re-resolve/re-authorize or establish a fresh authorized snapshot |
| `Expired` / `Cancelled` | Stop new effects, drain/abort outstanding leases, preserve durable outcomes |
| `OutcomeUnknown` | Reconcile using the same operation identity; do not blindly repeat external effects |

These are draft local API outcomes, not new numeric wire assignments. Remote failures disclose only
permitted coarse reasons; detailed policy facts stay in scoped diagnostics. Counters cover queue
depth, admitted/rejected work, arena and resident memory, generation pins, packet loss, useful
throughput, deadline misses, and measured/estimated/unknown energy/time. Full private graphs, key
material, and personal relationship lists do not belong in default logs.

## 9. Required evidence

- Allocation measurement on every hot success/error/cancel path, including malformed lengths,
  output exhaustion, generation changes, and full tables; parallel tests retain thread-local counters.
- Model-based lease tests covering double release, stale completion, cancellation during I/O,
  shutdown, worker failure, handle exhaustion, and reclamation with pinned old generations.
- Property tests for aggregate reservation conservation and checked arithmetic, including competing
  sessions, cold workers, nested scopes, and bytes already promised by flow control.
- Deterministic event replay with controlled clocks/entropy, loss/reorder, partitions and restarts;
  independent wire vectors are still required for protocol interoperability.
- Crash injection across inbox/application/receipt publication and revocation ordering; no duplicate
  application commit and no stale authority after recovery.
- Real adapter tests for bounded queues, close/drain, scheduler fairness, thermal unknowns, dependency
  closure, and platform memory behavior. None is a passing result merely because it appears here.
