# Library Construction and Ownership

## 1. Mandatory file discipline

Follow [AGENTS.md](../../../../../AGENTS.md) and the live repository instructions. New capabilities
are directory-backed libraries. A `mod.rs` declares modules and re-exports their API; state machines,
parsers, platform backends, receipt construction and tests belong in focused files.

| Signal | Required action |
|---|---|
| New implementation file | One cohesive responsibility; remain below 500 lines and split earlier if lifecycles mix |
| Existing file 500–1,199 lines | Record ownership review before substantial behavior; extract a cohesive component first |
| Existing file at least 1,200 lines | No new behavior without a tracked decomposition in the same programme |
| 1,400 lines | Escalation threshold, not the point to begin planning |
| Prose, vectors, generated tables or cohesive algorithm exception | Record why separation would reduce clarity; not permission for mixed lifecycles or hidden generated edits |

Every task brief identifies its exact files and responsibilities before editing. Do not solve a
size warning by moving the same monolith into `mod.rs`, creating `misc.rs`, or adding a generic
`manager.rs` containing unrelated state. Separate tests/fixtures where large; preserve public paths
through re-exports when extracting existing code. Generated code has a named producer and source.

## 2. Dependency direction

```text
client/qapp integrations
        -> qualia-peer facade and host selection
             -> core net/peer runtime and governed services
                  -> core net/qdnf protocol libraries
                       -> existing crypto, identity instruments, Webizen, Q42/storage
```

The core must never depend back on `qualia-peer`. Optional LIG, monetary adapters, GPU/LLM work and
foreign-stack bridges are feature-isolated. Networking consumes verified semantic/storage interfaces;
it does not own another ontology engine, mutable identity database or payment ledger.

## 3. Proposed directory ownership

Paths below are construction targets, not claims that the modules already exist. Adopt an existing
compatible directory instead of duplicating it; FND-01 records the final map.

| Owner | Target boundary | Focused files/responsibilities |
|---|---|---|
| Integration | crate/module roots, feature declarations, lockfile and public re-exports | One writer; consume reviewed export patches from other lanes |
| Core/Webizen | existing `governance/webizen/` | arena backing/leases; cache key/scope; bounded rule activation; VM adapter; focused tests |
| Core/query | existing `q42/volume/`, query and SPARQL owners | range quanta/cursors; bounded segment metadata; scoped query plans; explicit fallback admission |
| Storage | existing artifact/WAL/root owners | transaction intent; generation publication; recovery; reference/hold-aware GC; backend durability |
| Storage format | optional `container_qnf/` only after CORE-04 decision | `layout.rs`, `reader.rs`, `writer.rs`, `integrity.rs`, `validation.rs`, `tests/` |
| Crypto | existing crypto/key provider plus `net/qdnf/crypto/` adapters | suite types; key ownership; transcript stages; record proofs; bounded validation; vectors |
| Frame/bearer | `net/qdnf/frame/`, `bearer/` | header/extension codecs; checked ranges; reassembly; interface traits; each platform adapter separately |
| Adjacency | `net/qdnf/link/` | discovery; handshake state; neighbor table; rekey; teardown |
| Routing | `net/qdnf/route/` | realm/LSA validation; intra-realm planner; path-vector validation; hot forwarding; withdrawal/mobility |
| Resolution | `net/qdnf/resolve/` | DNI/RAR codecs; authority checks; source ordering; scoped cache; QSR semantic/exact rendezvous; alias selection |
| Sessions | `net/qdnf/session/` | handshake binding; packet spaces; recovery; congestion; streams; datagrams; migration; service dispatch |
| Semantic policy | `net/qdnf/contracts/` over existing fabric/modalities | bundle loader; codec; shape validation; supported rule compiler; temporal grant and purpose checks |
| Runtime | `net/peer/runtime/`, `resources/` | kernel step; events/effects; leases; reservation ledger; quotas/fairness; cancellation |
| Cell hosts | `net/peer/host/`, `cells/` | cell supervisor; IPC descriptors; ownership handoff; each OS backend; shutdown/restart |
| Facade | proposed `qualia-peer/src/` | typed public API; host builder; C/WASM adapters; examples; no duplicate protocol engine |
| Services | `net/peer/replication/`, `subscriptions/`, `custody/`, `compute/` | journal/checkpoints; projection planning; feeds; envelope storage; bounded RPC jobs; distinct receipts |
| Economics | `net/qdnf/economics/` and optional adapter packages | typed quantities; meters; work budgets; role offers; obligations; settlement intents; reconciliation |
| Evidence | core preservation owners plus narrow networking adapters | selection; holds; dependency closure; bundle assembly; custody; export; renewal; disposition |
| Legacy/transition | isolated gateway and optional bearer packages | DNS/Web/socket bridge; UDP/WireGuard/browser carriers; optional libp2p migration adapter |
| Verification | scoped `tests/`, `fuzz/`, `benches/` with repository conventions | independent peer oracle; vectors; fault harness; property suites; target runners; reports |

Directory subdivisions supersede the older illustrative flat `link.rs`/`session.rs` sketch when
that sketch would accumulate multiple responsibilities. Shared code moves downward to its actual
owner; copying codecs or rules between service and transport lanes is prohibited.

## 4. Allocation and state ownership

Write an ownership table for each module: input/output buffers, arena leases, mutation owner,
thread/process boundary, lifetime, byte/work cap and definitive release event. Include errors,
logging, cancellation, old/new generations, and borrowed file pages. `Arc` or a process boundary
does not make hidden allocations free.

Tier-1 data is flat and caller-buffered. Large arrays belong in caller-provisioned backing storage,
not an unbudgeted giant thread stack. Cold builders use a declared bounded workspace and publish
flat immutable results. No borrowed pointer crosses IPC; use validated range/epoch descriptors.
No network-packet arrival can grow a rule vector or construct an unadmitted Webizen arena.

## 5. Shared-file protocol

Workers own assigned implementation files. Only the named integrator edits shared crate roots,
Cargo manifests/lockfile, registry/profile bundles and the task register. Workers submit exact
re-export/dependency changes as handoff artifacts. The integrator reads existing content and applies
minimal patches; it never replaces whole shared files from a worker snapshot.

Code extraction precedes adding behavior to an oversized owner. The task records old/public paths,
new responsibilities, compatibility fixtures and which tests moved. Do not rewrite unrelated code
or discard concurrent edits while repairing an adjacent networking requirement.

## 6. Temporary artifacts

Use scoped, byte-budgeted RAII temporary directories for tests, conversion, captures and generated
fixtures. Promote validated retained evidence into an explicitly selected artifact directory.
Clean only marker-verified owned directories under the resolved parent; never recursively sweep
workspace/system temp. Cover success, error and unwind. Compilation/benchmark outputs follow the
repository's shared target policy; one agent must not allocate another full build tree by default.

## 7. Review questions before accepting a patch

The reviewer checks one responsibility per file, explicit lifetimes, bounded public output,
dependency direction, no copied engine, and measured hot-path allocation behavior. Review generated
changes separately. Any size exception has a scoped written rationale; breaking a component into
small files does not by itself prove good ownership or correctness.
