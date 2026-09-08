# Peer Runtime, Cells and Parallel Ownership

Implement the [runtime model](../../peer-runtime-api.md) and [cell design](../../network-cell.md) using the verified Webizen/core boundaries. Each package uses separate library files for planning, hot steps, I/O and recovery.

All packages follow [library boundaries](../library-layout.md), [swarm handoffs](../swarm-protocol.md)
and the [shared validation standard](../validation-matrix.md). Package dependencies are authoritative
in the register; early interface-based work cannot bypass their completion evidence.

## RT-01 — Lease kernel and atomic resource admission

**Owner and inputs:** Runtime owner; reviewed interfaces and CORE-01 arena contract.

**Construction boundary:** net/peer/runtime and resources libraries: handles, leases, ledger, events/effects, scheduler, cancellation and tests.

- [ ] RT-01.01 Define nonwrapping generation handles and exclusive input/output lease ownership; reject stale and double-release operations.
- [ ] RT-01.02 Implement bounded events/effects with consuming completion APIs and explicit retained-buffer ownership.
- [ ] RT-01.03 Atomically reserve all applicable host/cell/role/session/operation scopes or leave every counter unchanged.
- [ ] RT-01.04 Charge shared pages once physically at the host and against each consumer's working set; no hidden scratch, stacks or allocator overhead.
- [ ] RT-01.05 Back advertised flow credit and queued work with actual capacity including fragments, retransmits and unconsumed application events.
- [ ] RT-01.06 Separate transient/pre-auth budgets from verified peers; Sybil identities must not multiply global admission.
- [ ] RT-01.07 Reserve bounded closure, revocation, completion and reclamation progress so data pressure cannot deadlock release.
- [ ] RT-01.08 Implement deterministic fair scheduling, work quanta, deadlines and bounded crypto/compiler queues.
- [ ] RT-01.09 Define cancellation, timeout, late result, worker fault and definitive buffer release without freeing live I/O/DMA references.
- [ ] RT-01.10 Publish immutable route/policy generations and order current-authorization checks with delivery and durable commit.
- [ ] RT-01.11 Prevent cell/job restart or path migration from resetting operation identity or financial/resource reservations.
- [ ] RT-01.12 Measure no-allocation success/error/cancel behavior and property-test reservation/lease conservation.
- [ ] RT-01.13 Verify saturation of every queue/table, generation exhaustion and unresponsive consumers with reproducible fault traces.
- [ ] RT-01.14 Publish public resource/error semantics and capacity sizing evidence; avoid a giant kernel manager file.

Exit: owned resources remain conserved under failure, cancellation and concurrency. Handoff to cells, facade, economics and services.

## RT-02 — Supervised cells and enterprise parallel execution

**Owner and inputs:** Runtime/platform owner; coordinate existing WorkerCell and scheduler maintainers.

**Construction boundary:** cells and host libraries with distinct supervisor, IPC, flow handoff, Webizen lease assignment and platform enforcement adapters.

- [ ] RT-02.01 Integrate existing cell abstractions under explicit ownership; do not create a third incompatible scheduler.
- [ ] RT-02.02 Reserve a cell before construction, ordinarily at most 512 MiB, plus host aggregate overhead and simultaneous worker leases.
- [ ] RT-02.03 Assign reusable Webizen arena/workspace locally or to policy cells; smaller network profiles cannot silently allocate a full 42 MiB arena they cannot afford.
- [ ] RT-02.04 Separate 42 MiB complete-pass, arena backing, cell and OS memory measurements; enforce all claimed limits on each backend.
- [ ] RT-02.05 Keep exceptional LLM/similar profiles explicit and isolated from ordinary networking and essential reserves.
- [ ] RT-02.06 Use fixed-capacity IPC descriptors with validated range/epoch/purpose/authority; no cross-process pointers or mutable validation/use races.
- [ ] RT-02.07 Partition bearers/flows and bounded read/verify jobs across cells, with one mutable owner per session and deterministic reductions where required.
- [ ] RT-02.08 Implement quiesce/handoff of flow state, counters, leases and replay/nonce ownership; reject duplicate or stale completions.
- [ ] RT-02.09 Apply Linux/Windows/other claimed host controls with measured process-tree/shared-memory semantics; report unavailable controls honestly.
- [ ] RT-02.10 Supervise crash/restart with fresh online keys, durable operation recovery and quarantine until shared memory is definitively inaccessible.
- [ ] RT-02.11 Bound CPU/work as well as memory; include thermal and unavailable sensor behavior without treating unknown as safe/free.
- [ ] RT-02.12 Test 1/2/4 and larger admitted cell counts, uneven load, failed workers and host exhaustion with a fixed workload.
- [ ] RT-02.13 Measure useful throughput, contention, tail latency, I/O bandwidth, energy and memory; no linear-scaling claim from cell count alone.
- [ ] RT-02.14 Publish supported embedded/process/WASM boundaries, actual capacity limits and recovery evidence.

Exit: enterprise networking uses real bounded parallel cells and the same graph core; no cell multiplies authority, funding or host capacity.

## RT-03 — Public facade and governed service entry points

**Owner and inputs:** Runtime API owner with application maintainers.

**Construction boundary:** Proposed qualia-peer facade, typed service handles, explicit host builder and C/WASM boundaries; core has no reverse dependency.

- [ ] RT-03.01 Expose discover/dial/open/stream/datagram/cancel APIs over the same QDNF kernel for native and selected transition hosts.
- [ ] RT-03.02 Bind service handles to current authority, semantic profile and operation context; raw transport reachability never bypasses policy.
- [ ] RT-03.03 Integrate bounded source ordering and at most the selected dial-race budget; clean up losing and late attempts.
- [ ] RT-03.04 Keep one application operation across path change/retry with explicit outcome uncertainty and no duplicate external effect.
- [ ] RT-03.05 Build bounded async wrappers without hidden runtimes, unbounded channels or per-packet task creation.
- [ ] RT-03.06 Implement C/WASM buffer/handle ownership, cancellation and error mapping; test wrong ranges, lifetime misuse and unsupported targets.
- [ ] RT-03.07 Verify feature/dependency closure: native facade excludes libp2p and unwanted DNS/IP, GPU/LLM and settlement dependencies.
- [ ] RT-03.08 Provide runnable minimal examples for native peers and governed service calls with exact build/feature instructions.
- [ ] RT-03.09 Run two-application native exchange with IP disabled, valid authorization and negative delivery cases.
- [ ] RT-03.10 Publish API/version/migration notes and measured dependency/binary costs; an ergonomic facade alone does not make the existing core small.

Exit: applications use the independent runtime through a stable, bounded API. Handoff to SVC, ECO, OPS and qualification.
