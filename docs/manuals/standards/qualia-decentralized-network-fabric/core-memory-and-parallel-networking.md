# Webizen, Core Memory and Parallel Networking

**Status:** Source-backed architecture review; proposed integration, no runtime changes
**Date:** 2026-09-07

## 1. What the code establishes

**Webizen is the Sentinel.** Its governance VM and SLG tabling arena live in
[`governance/webizen`](../../../../crates/qualia-core-db/src/governance/webizen/mod.rs).
The arena, an execution cell and an entire persistent graph are different scopes. The source
confirms a 42 MiB table and 512 MiB worker-cell declarations; it does not establish that every cell
already owns exactly one arena or that those declarations enforce all live memory.

| Source | Observed implementation | Implication |
|---|---|---|
| [Webizen constants](../../../../crates/qualia-core-db/src/governance/webizen/mod.rs) | `SLG_ARENA_SIZE = 42 * 1024 * 1024`; 48-byte Quins; 917,504 slots; separate 512-entry recent-slot ring | 44,040,192 bytes of Quin slots per constructed arena, plus supporting state |
| [SlgArena construction](../../../../crates/qualia-core-db/src/governance/webizen/arena.rs) | `Vec::with_capacity(MAX_SLOTS)` followed by prefill; two additional rule vectors and fixed recent-slot storage | The main table is allocated once, but construction is not zero-heap; full-object/worker memory exceeds table bytes |
| [Table writes/lookups](../../../../crates/qualia-core-db/src/governance/webizen/arena.rs) | Slot chosen by hash of subject/predicate/object; collisions overwrite; `head_pointer` advances without choosing the slot | A direct-mapped memoization table with recent-write tracking, not a lossless FIFO log |
| [VM entry](../../../../crates/qualia-core-db/src/governance/webizen/vm.rs) | `execute_vm_frame` borrows `&mut SlgArena`; `CheckTable` can return a cached result | VM invocation does not allocate or select a per-cell arena by itself; caller ownership and cache scope matter |
| [RuleEngine](../../../../crates/qualia-core-db/src/modalities/logic/rules.rs) | Owns an arena; constructors create one; authoring/results also use vectors/strings | An arena can belong to an engine instance, independently of a worker cell |
| [Local worker cells](../../../../crates/qualia-core-db/src/platform/local_scheduler.rs) | `memory_boundary = 512 * 1024 * 1024`; worker has channels/affinity but no arena field; `process_job` simulates offset advancement | 536,870,912-byte declared boundary, not a measured allocator cap or complete Webizen execution integration |
| [Daemon worker cells](../../../../crates/qualia-core-db/src/services/daemon_swarm.rs) | Separate `WorkerCell` with 512 MiB field, attached-block vector and several maps | Another ownership abstraction to reconcile; its name is not proof of bounded memory or shared scheduler semantics |

Other call sites construct temporary arenas for agreement checks, inference governance, ingestion
and MCP operations. Therefore “one 42 MiB buffer per cell” is a sensible target ownership policy,
but is not an invariant demonstrated by these implementations. No process RSS/allocation benchmark
or runtime test was run for this source review.

## 2. Ring and cache semantics matter

`write_table` overwrites the hash-selected slot regardless of age. A separate ring stores recent
slot indices. `collect_active_quins` examines only that bounded recent history and caller output;
it does not enumerate every fact in a large persisted graph. The FIFO wording in the method comment
does not describe the slot-selection code.

The inspected table key/check compares subject, predicate and object, omitting context, policy
generation, requester and expiry. A raw cache hit cannot serve as a scoped QDNF authorization proof.
Integration must isolate arenas by an adequate scope or extend and verify the cache binding,
revalidating current authority. It must also reconcile the existing four-field parity checks with
canonical five-field persistence, already tracked in P15. No stale or cross-context table hit may
deliver private data, discharge an obligation or satisfy a preservation claim.

Eviction from a computation cache is acceptable only with defined recomputation/continuation
semantics. It cannot erase an in-flight network commitment, retransmission lease, replay identity,
causal tombstone or held evidence. These remain in their own bounded owned tables or durable core
records. An evicted premise must not be interpreted as evidence that it never existed.

## 3. The zero-heap contract versus current construction

The target remains caller-owned fixed storage for Tier-1/ABI/Sentinel work: no `Vec`, `String` or
`Box` in those paths, including errors. A preallocated vector with allocation-free steady writes
does not satisfy a blanket claim that construction is allocation-free. The source must be described
accurately rather than treating its comments as proof of conformance.

Rule registration, staging and activation push into vectors, so the existing path also needs a
bounded capacity/admission audit. Construction outside a loop is not automatically exempt: use the
repository's actual Tier-1/Tier-2 classification, caller buffers and total budgets. The networking
integration should receive reusable arena/table storage from the cell's owner, plus bounded rule
slices and scratch, and reject insufficient capacity before mutating state.

The repository's 42 MiB complete-pass requirement is stronger than this table-size constant.
Keep separate accounts for reserved backing capacity and the pass's live referenced working set.
If a pass touches all 42 MiB of slots plus scratch, it cannot claim a complete 42 MiB working-set
bound. Partition work, admit a smaller active window, or reduce slots in a reviewed arena profile;
do not quietly relabel overhead. This review changes no arena constant or ABI. The encompassing
ordinary cell still has its independently accounted ceiling of at most 512 MiB.

## 4. Large graphs and network state use the same core

A graph or ontology hundreds of gigabytes in size does not require an equally large resident
arena. [Q42 range volumes](../../../../crates/qualia-core-db/src/q42/volume/range_volume.rs) provide
caller-buffered query pages and continuation state over a range source;
[block cursors](../../../../crates/qualia-core-db/src/q42/volume/cursor.rs) decode sequential blocks
into caller storage. [Manifest/publication owners](../../../../crates/qualia-core-db/src/q42/volume/manifest.rs)
represent persistent segments separately from current working pages.

Network facts are graph facts and should use that same semantic, indexing, provenance and
bounded-access machinery. Large data size alone is no reason to define QNF or a second database.
The [QNF candidate](../qnf-network-container-draft.md) must justify only a specialized access/layout
benefit for exact objects and reusable views compared with existing core facilities. The semantics,
memory policy and ability to process large datasets do not depend on choosing a new format.

Streaming a storage scan does not prove complete bounded evaluation of an arbitrary ontology.
Joins, global constraints, closure and recursion can require additional intermediate state and
cross-partition communication. Use bounded frontiers, indexed partitions, explicit spill quotas and
resumable checkpoints where the selected evaluator supports them. Bind cursors to immutable source
and policy generations. No fixed recent-fact window or output truncation can be reported as a complete
global result; return continuation, incomplete/unsupported or resource-limit outcomes as appropriate.

Network-specific differences are primarily lifetimes, deadlines and adversarial arrival: packet
buffers, timers and congestion state have different ownership from historical graph records.
Reuse core representations where their guarantees fit and expose explicit adapters for those
lifecycles. This is specialization over the same graph engine, not a claim that network facts are
intrinsically too large or too complex for Q42.

## 5. Enterprise scaling through cells

### 5.1 Existing scale mechanisms and remaining bounds

Further source inspection confirms cross-segment query continuation and joins, with specific limits:

| Existing path | Reuse and required boundary |
|---|---|
| [Range volume-set open/query](../../../../crates/qualia-core-db/src/q42/volume/manifest.rs) | Opens manifest/segment metadata into vectors; payload paging alone does not bound startup metadata. Budget decoding and use a bounded segment-handle cache for large partition counts. |
| [Range query loop](../../../../crates/qualia-core-db/src/q42/volume/range_volume.rs) | A small output page can scan all remaining blocks when matches are sparse/absent. Add explicit block/work/time quanta and resumable progress; output capacity is not a work limit. |
| [Range hash joins](../../../../crates/qualia-core-db/src/sparql_library/range_hash_join.rs) and [executor](../../../../crates/qualia-core-db/src/sparql_library/sparql_executor.rs) | Caller-sized hash tables and a nested-loop fallback support cross-segment work. Bounded memory does not make nested-loop latency bounded; general partitioned join spill is separate work. |
| [CLI resident selection](../../../../crates/qualia-cli/src/sparql.rs) and [compatibility reader](../../../../crates/qualia-core-db/src/q42/q42_reader.rs) | Selection uses the physical root-file size, while a manifest root can expand to all segments in a vector. Network admission must check logical decoded size and forbid unbudgeted whole-graph fallback. |
| [External sorter](../../../../crates/qualia-core-db/src/sparql_library/external_sort.rs) | Real disk runs and 32-way bounded merging exist, but one million Quin slots reserve 48,000,000 bytes, about 45.8 MiB, before overhead. Reuse with an explicit smaller budget; do not claim this is already a 42 MiB/no-Vec path or general reasoning spill. |
| [Daemon shard creation](../../../../crates/qualia-core-db/src/services/daemon_swarm.rs) | Appends a new worker cell without reserving aggregate host memory. Cell count must be admitted before construction; a declared 512 MiB field is not enforcement. |

These findings refine reuse requirements; they do not remove the existing working graph primitives
or justify duplicating them in a separate networking engine.

### 5.2 Parallel execution strategy

An enterprise router agent can assign parallel work across multiple ordinary cells, each at most
512 MiB, with Webizen evaluation resources included in the applicable ownership budget. Cell count
is admitted from actual host memory, CPU/work capacity, I/O and control reserves, not merely from
logical CPU count. Memory capacity does not imply linear throughput or sufficient network bandwidth.

| Work | Parallelization boundary | Shared invariant |
|---|---|---|
| Ingress and session processing | Bearer queues or stable flow partitions with one mutable owner | Receive credit, replay/nonce state and leases cannot be duplicated |
| Verification and semantic preparation | Bounded jobs over immutable source ranges and rule profiles | Results bind exact inputs, scope, generation and current authority |
| Route/view construction | Partition candidate evaluation, then deterministic bounded reduction | Loop prevention, withdrawal ordering and one published generation |
| Large-graph queries and QSync | Indexed disjoint ranges plus explicit continuations/joins | Authorized completeness, snapshot consistency and causal/replay identity |
| Evidence and economic records | Partitioned preparation with serialized durable publication boundaries | Holds, obligations and parent reservations survive failure or reassignment |

Allocate a cell's reusable Webizen arena lease explicitly. A host may provide separate policy cells
or permit local evaluation in the networking cell; it does not need a second semantic engine for
networking. The service interface describes the same authority/limits in either placement. Parallel
passes receive separate bounded leases; never share mutable arena state without an ownership protocol.

Use fixed-capacity queues, backpressure and caller-owned output partitions, with deterministic
merge/commit order where semantics require it. Quiesce or transfer flow ownership before rebalancing.
New cell epochs invalidate stale completions; durable operation IDs, role budgets and preservation
holds do not reset. Additional cells cannot evade an agent's energy, time, typed compute or spending cap.

Exceptional LLM/similar execution remains explicitly classified and isolated; its allowance does
not enlarge ordinary routing cells or consume their essential control reserve. Memory for shared
file pages, NIC/kernel buffers, worker stacks and old/new generations remains visible at host level.

## 6. Integration evidence required

P15/P17 in [Implementation and Conformance](./implementation-conformance.md) cover arena scope/parity,
actual per-cell ownership, fixed caller storage, bounded rule activation, aggregate reservations and
queue/flow handoff. Validate eviction/recomputation and cross-context cache cases before placing the
current Webizen helpers at a network authorization boundary. Measure zero-allocation behavior on
success/error/cancel paths and actual platform memory enforcement.

Exercise a dataset substantially larger than available RAM through supported paged operations,
checking complete results against an oracle or reporting explicit continuation/incompleteness.
Measure 1/2/4 and larger admitted cell counts with fixed workloads: useful throughput, latency,
contention, disk/network bandwidth, memory, energy and compute. These are future evidence requirements,
not results inferred from the ring-buffer, 512 MiB field or a large addressable file length.
