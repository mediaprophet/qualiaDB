# Qualification and release evidence

Revision 1, 12 November 2026. These are required future checks, **not executed streaming results**. The existing 61 format and 33 Etalon tests reported by the format standards are regression baselines only. No implementation-completion claim follows from this plan.

## 1. Mandatory R1 gates

| Gate | Acceptance evidence | Owning packages |
|---|---|---|
| G01 Native independence | Two real peers discover/admit/transfer a useful partial object over supported native bearers with HTTP, DNS, IP sockets and libp2p absent. A fixture that calls the verifier directly does not count. | 04, 09, 17 |
| G02 Binding parity | Native, HTTP and WSS yield identical verified-byte identity, policy decisions, cancellation outcome and idempotent receipt effects for the same fixtures. Transport diagnostics/timing may differ. | 09, 14, 26 |
| G03 PQ authority | Primitive/protocol vectors, independent paired-COSE interoperability, trusted root/rotation/revocation, transcript/key confirmation, downgrade and 0-RTT refusal. External composition review with findings resolved. | 03, 04, 10 |
| G04 Bounded metadata | Duplicate keys, indefinite/oversized CBOR, trailing bytes, page-count attacks, bad lengths/offsets, u64/usize overflow, invalid flags, legacy hash lengths and metadata overlaps rejected before excess allocation. | 02, 06, 07 |
| G05 Range proofs | Independent tree implementation and every index/context/entry/generation/final-length/sibling mutation; empty and non-power-of-two cases; verified leaves never imply full container validation. | 05, 12, 17 |
| G06 Encryption lifecycle | Nonce uniqueness under retries/crashes, acyclic context construction, wrong-key/tag/proof failures, no early plaintext, confidential metadata, recipient wrap/rotation and usage-limit refusal. | 03, 10 |
| G07 Reservations/cancellation | Zero Rust heap allocation on hot success and rejection with positive-control allocator tests; measured stack/scratch/aggregate queues; generation-fenced late completions; control reserve preserved. | 08, 09, 13 |
| G08 Durable resume | Crash injection at every data/flush/bitmap/promote boundary; sparse holes, torn writes, external mutation, full disk, lost writer exclusion, quota and evidence-hold cases. | 11, 15 |
| G09 Governance/disclosure | Forged parity token, unsigned VC proof, stale policy/semantic closure, forbidden representation switching, restricted index/gap leakage and prohibited route all refuse. Payment cannot bypass consent. | 07, 12, 16 |
| G10 Commons accounting | Exemption/sponsor cases, >16 operations, persistent replay protection, split/coalesced rounding, cancel/retry/provider failover and exact cap conservation. No packet-ACK billing. | 16 |
| G11 HTTP/browser | 206/200/416, truncation/extra bytes, gzip transformation, validator changes, redirects, CORS hidden headers, forbidden headers, OPFS denial, missing SAB, tab closure and WebGPU/device limits. | 14, 15 |
| G12 WebSocket | Authentication/Origin misuse, fragmentation/compression, send/receive queue floods, credit violations, disconnect/reconnect and protected-control/PQ boundary. Browser residual allocation limitations documented. | 26, 15 |
| G13 Consumers | Useful partial 10D/EMF/M64/Q42 selected profile; exact repaired trailer checks; complete-entry P64 and governed Etalon lifetime; no sparse/model shortcut. | 12, 13 |
| G14 Independent release | Separate encoder/verifier/peer, platform evidence, fuzz/fault results, reproducible performance and security review tied to integrated source state. | 17, 23, 25 |

Each gate has positive controls: demonstrate that the test detects an intentionally faulty implementation/fixture, then remove that fault. Invalid inputs must fail with bounded work and stable error classes, not crashes or quiet fallback. No benchmark or success demo compensates for a failed security gate.

## 2. Corpus and independent implementations

Build synthetic public fixtures: empty and one-entry QBDL; mixed alignment 64/4096; non-power-of-two leaf counts; duplicate/overlapping inventory; 10D typed dependency closure; full-hash EMF/M64 with a corrupted digest tail; Q42 segment; P64 v4 with 16 KiB pages and ETL metadata; encrypted entries with separately authorized metadata; mixed-rights source and public projection. Include large virtual/sparse offsets near 32/64-bit limits without allocating a terabyte.

Use an independent small reference encoder/verifier for descriptor and tree bytes. At least one peer path must not import the same production proof/parser implementation. Compare exact canonical bytes and accepted/refused cases; another process with identical library code is only process interoperability. Primitive vectors come from the selected standards/provider with version/provenance recorded; generating and verifying with the same implementation is insufficient.

Fuzz boundaries separately: descriptor/index, proof, AEAD framing, service messages, storage recovery and HTTP/WebSocket adapters. Then run stateful campaigns for revoke/cancel/rekey/provider failover and duplicate operations. Keep seed corpora, crash reproducers, source hashes, limits and run durations. Sanitizers/platform tooling are selected by availability; unsupported tools are reported, not treated as passes.

## 3. Platform and carrier matrix

| Target | R1 evidence |
|---|---|
| Windows native | Publisher/client/provider, real native path, file lease/exclusion, HTTP/WSS, CPU consumer; DX12 only if tested |
| Linux independent peer | Real native interop, independent proof/parser path, durable restart and HTTP/WSS; Vulkan only if tested |
| Browser/WASM | Named versioned engine(s), dedicated worker, OPFS and bounded fallback, HTTP/WSS, CORS/COI tests; WebGPU separately reported |
| macOS/Metal | Explicit unsupported/unqualified until platform run; do not infer from Windows/Linux |
| Constrained/no-GPU | 8-slot/8-MiB profile and CPU admission/refusal; no unlimited fallback |
| WebRTC | R2 QBS-27 gate below, not inferred from WSS or native success |

Pin builds/features and actual dependency locks. Preserve existing package-level test requirements, including Etalon's configured test workflow; a focused cargo test is useful evidence only for its actual scope. Existing conformance harness command remains `cargo test --manifest-path tools/container-format-conformance/Cargo.toml --target-dir target-container-standards --offline` from QualiaDB. Do not rebuild into a live Synth MCP target. New harness commands are added by QBS-17 and verified before documentation claims they work.

## 4. Performance and resource evidence

Measure cold/warm descriptor verification, first useful admitted closure, sustained verified throughput, proof/outboard overhead, CPU and peak resident/stack/scratch, disk writes, VRAM staging, cancellation waste and fairness under competing streams. Compare equivalent security and fixture sets; raw unverified throughput is context, not a competing security implementation. Report hardware, compiler, algorithms/features, cold caches, sample count and p50/p95 rather than a single best run.

Initial demonstration target: obtain a useful admitted closure of at most 512 KiB from a synthetic bundle of at least 8 MiB without fetching the complete bundle, and show bounded residency independent of total extent. All advertised limits must hold under malformed inputs and concurrent tasks. QBS-00/01 set hardware-specific latency/throughput budgets before optimization; do not invent universal millisecond claims or revise targets after a failed run without a decision record. Crypto failure storms, uncooperative HTTP/WSS senders and GPU contention are part of the benchmark workload.

Privacy assessment covers metadata names/hashes, proof request patterns, timing/size, cache existence probes, relay disclosure and logs. Optional padding/cover traffic consumes an explicit budget; it does not imply anonymity or ORAM. Telemetry aggregates locally where possible and never stores session keys, DEKs, private solver maps or unrestricted behavioral histories.

## 5. R2/R3 feature gates

| Feature | Additional release evidence |
|---|---|
| Generations/sensing, QBS-18 | Signed sequence/predecessor forks, retained-history gaps, clock/calibration changes and consent revocation; no false append-consistency proof |
| Escrow, QBS-19 | Explicit model of outstanding rights, acknowledged return/fencing, crashes, arbitrary partitions, duplicate receipts and clocks; independent economics review before collecting recovery fees |
| Solver Commons, QBS-20 | Exact-key round trips, false-equivalence counterexamples, poisoned witnesses, numerical overflow/tolerance, topology disclosure and controlled QPU cost authorization |
| Selective inference, QBS-21 | Correct metadata/operator dependencies, alignment and excluded-writer bytes; KV/precision switching boundaries; quantitative correctness against complete-entry baseline |
| Alignment, QBS-22 | Full old/new reader/writer matrix, malformed flags, placement benchmarks and versioned migration |
| HLS/DASH, QBS-24 | Actual audiovisual player interoperability, permissions/provenance and discontinuity handling; no arbitrary-format claims |
| WebRTC, QBS-27 | Two independent browser peers; reliable ordered data, direct/relay ICE, privacy restrictions before gathering, authenticated signaling substitution/replay, glare/restart, maxMessageSize fragmentation, buffering and consent loss; same security outcomes as native/HTTP/WSS |

R2 completion requires its own release record; finishing QBS-25 certifies R1 only. Features may remain unavailable while their prerequisites are unresolved. An external review rejection keeps the relevant gate closed and is recorded with its concrete finding and remediation.

## 6. Evidence record

For each run record package/gate, source commit plus dirty fingerprints, interface/vector hashes, model/author/reviewer, target/features, exact command, exit/result, counts, allocations/resources, source fixture provenance, exclusions and reproducible artifact paths. Keep secret-bearing artifacts out of evidence. Registry `complete` requires accepted evidence plus integration results. Planning checks verify the registry DAG and document links only; they cannot satisfy protocol gates.
