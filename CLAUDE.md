# QualiaDB — AI Agent Orientation

Read this before writing any code. It exists to prevent systematic misconceptions.
The detailed coordination document is [`AGENTS.md`](AGENTS.md).
The architecture reference is [`ARCHITECTURE.md`](ARCHITECTURE.md).

---

## 1. The LLM Engine Is NOT Ollama

This is the most common and most damaging mistake. Qualia has its own native, in-process
LLM inference stack. There is no Ollama, no llama.cpp HTTP server, no Python runtime,
and no external daemon to query.

| What you might assume | What actually exists |
|-----------------------|----------------------|
| Ollama / llama.cpp HTTP server | `gguf_bridge.rs` — native GGUF loading via `memmap2` |
| Python runtime or subprocess | Pure Rust, in-process, zero Python |
| External daemon on some port | The Qualia daemon on 4242 is the *graph engine*, not an LLM server |
| `POST /api/generate` | `AgentRuntime::infer()` — a Rust trait call |
| Model pulled from a registry | GGUF file mapped directly from disk via OS page cache |

**The actual inference stack:**
1. `gguf_sharder.rs` — parses GGUF header, generates `NQuin` pointer map (byte offsets encoded into quin object field, upper 4 bits = modality flag `0b1001`)
2. `gguf_bridge.rs` — maps model weights into the OS page cache with `memmap2` (zero heap allocation); dispatches fused transformer blocks to the GPU
3. `shaders/fused_tensor_contraction.wgsl` — WGSL compute shader, 64 threads/workgroup, 4096 FMA ops per thread; runs on DirectML / Vulkan / Metal / WebGPU via `wgpu`
4. `llm_agent.rs` — `LocalLlmAgent` orchestrates the two-thread Phase 8 bifurcated compute (see §3 below)
5. `orchestrator.rs` — `TaskOrchestrator` manages `ModelLifecycle` state machine and `ThermalGovernor`

---

## 2. The Three Backend Modes

`AgentBackend` in `llm_agent.rs` has exactly three variants:

```rust
Local   // GGUF on-disk → wgpu → in-process. No outbound traffic. 128 MB RAM cap.
Remote  // API call → Nym mixnet → ILP metered. Requires signed VC from Principal.
Hybrid  // Local-first. Falls back to Remote only with explicit Principal consent.
```

Do not add an Ollama backend. Do not add an HTTP client to an external model server.
If you need a new backend, model it on `LocalLlmAgent` in `llm_agent.rs`.

---

## 3. Phase 8 Bifurcated Compute

Token generation is not a simple loop. It uses two wait-free SPSC ring buffers (`rtrb`):

```
LLM Engine thread  ──logits──►  LogitStream  ──►  Webizen Sentinel thread
                   ◄─control──  ControlStream ◄──  (detects anomalies; can DenyRollback)
```

The Sentinel reads logit vectors in real time. If it detects an anomaly (e.g., 0x99
byte signature for anachronism), it injects a `DenyRollback` into `ControlStream` and
the LLM recalculates. This happens **mid-generation**, not post-hoc.

Do not replace this with a simple `generate() -> String` wrapper. The bifurcation is
the governance mechanism.

---

## 4. The Webizen VM Gates Every LLM Call

`orchestrate_inference()` in `orchestrator.rs` always runs:

1. `validate_intent(intent)` — pre-flight. Checks N3Logic Rights Ontology rules. If `Deny`, writes a conduct violation Quin to the WAL (signed with ed25519) and aborts. The model is never invoked.
2. `agent.infer(prompt, graph_context)` — the actual GPU inference.
3. `validate_output(output)` — post-flight. Output must have ≥1 provenance `NQuin` citation. Ungrounded output is rejected.

This is **mandatory infrastructure**, not optional middleware. Do not bypass or stub it.

---

## 5. The Daemon on Port 4242 Is the Graph Engine

```
localhost:4242  =  Qualia semantic graph daemon
                   Endpoints: /health, /query (SPARQL-style over quins)
                   Chat relay: /chat/publish, /chat/pull
                   WebTorrent: /torrent/webseed/{hash}, /torrent/seed, /torrent/telemetry
                   NOT an LLM server
```

The benchmark harness (`benchmarks/qualia/runner.py`) queries this daemon to measure
point / two-hop / filter latency on the graph. The LLM inference runs entirely separately,
inside the same process as the graph engine.

---

## 6. Core Invariants (from AGENTS.md §0)

These break things if violated:

| Rule | Why it matters |
|------|---------------|
| No `Vec`/`String`/`Box` in hot paths | Breaks zero-copy ABI used by WASM, desktop, and edge targets |
| 48-byte `NQuin` for all semantic data | Everything is bit-packed into 6 × `u64` fields |
| 42 MB `SlgArena` ceiling | The Webizen VM must fit within this; allocating past it is an OOM |
| `q_hash()` for all URIs | No runtime string allocation; FNV-1a at compile time |
| Opcodes `0x10+` for new modalities | `mini_parser.rs` owns `0x00–0x04`; never overlap |

---

## 7. What to Read First for Common Tasks

| Task | Start here |
|------|-----------|
| Modifying inference | `llm_agent.rs`, then `gguf_bridge.rs`, then `gguf_sharder.rs` |
| Adding a logic modality | `AGENTS.md §3` + `deontic_logic.rs` as template |
| Touching the graph engine | `orchestrator.rs`, `storage.rs`, `wal.rs` |
| MCP server changes | `mcp_server.rs` |
| Flutter FRB API changes | `crates/qualia-flutter/rust/src/api/qualia_api.rs`, then run `flutter_rust_bridge_codegen generate` |
| Group chat / sub-agents | `chat_agents.rs`, `chat_relay.rs`, `chat_inference.rs` in `qualia-client-core` |
| Ontology workbench / seeding | `ontology_workbench.rs`, `webtorrent_seeder.rs`, `webtorrent_routes.rs` |
| Benchmark harness | `benchmarks/harness.py`, `benchmarks/qualia/runner.py` |
| Governance / rights | `webizen.rs`, `agency.rs`, `deontic_logic.rs` |
| Profile / identity | `profiles.rs`, `key_vault.rs`, `identifier.rs` |
| Scientific primitives | `webizen.rs::execute_vm_frame` (fully wired, not stubs) |
| Specialized domain libs (matrix, stats, ML, finance, medical, physics, chemistry, engineering) | `specialized_libs/` — all 9 active (79 tests); each `*Library::new() + initialize()`; MCP-exposed via `mcp_server.rs` |
| Cross-platform storage (ZNS / APFS / WinNVMe / Mmap) | `storage_driver.rs` — use `open_storage(data_dir)`, not `ZnsZoneManager::new()` directly |
| Thread QoS / CPU placement | `platform_scheduler.rs` — `bind_inference_thread()` / `bind_background_thread()` |
| Network filtering (eBPF / WFP / XPC / VPN) | `ebpf_filter.rs` — use `open_platform_filter()` |

---

## 8. Known Inaccuracies to Watch For

- `ARCHITECTURE.md §5` previously said "llama.cpp" — **corrected** 2026-06-06. The backend is `wgpu`, not llama.cpp.
- `logic.rs::Always/Eventually/Next` opcodes are **not** real LTL operators — they compare a float threshold on a single Quin. Use `temporal_ltl.rs::evaluate_ltl_trace` instead. See `AGENTS.md §4-B`.
- `logic.rs::extract_float` uses `0b001 << 60` as an f32 tag, conflicting with `resolver.rs` which uses the same bits for `xsd:integer`. See `AGENTS.md §4-D`. Do not "fix" this unilaterally.
- `infer_local_model()` in `llm_agent.rs` runs a real Phase 8 autoregressive loop through the GPU layer with real `token_embd.weight` lookup via `GgufTensorIndex::dequantize_token_embedding_into` (host targets). WASM still uses the mock ring-buffer path.
- The `qualia_api.rs` comment on `check_ollama_status()` is a legacy stub. Qualia does not use Ollama. The function always returns `false`.
- `fiduciary_crypto.rs` is **real ML-DSA-65 (FIPS-204) via the `fips204` crate** as of 0.0.12. It previously contained a SHA3-based *simulation* of ML-DSA — that fake lattice path has been removed, and the serialized key/signature byte layouts changed (1952-byte pk / 4032-byte sk / 3309-byte sig). If recall/older docs describe it as "simplified for demonstration", that is stale.
- `specialized_libs/cryptographic_library.rs` real primitives: Ed25519 (sign/verify for non-MLDSA keys), **ML-DSA-65** (for `KeyAlgorithm::MLDSA`, via `fiduciary_crypto.rs`), AES-256-GCM + **ChaCha20-Poly1305 + XChaCha20-Poly1305** (AEAD), SHA-256/SHA-512 + **BLAKE3** (hashing), **HKDF-SHA256** (KDF). Still scaffold-only (enum variants without backends): Kyber/NTRU/SPHINCS, RSA/ECDSA, and the zk-SNARK/Groth16/PLONK proof types (`generate_proof_data` is a SHA-256 commitment, not a real proof). See `CRYPTO_IMPLEMENTATION_PLAN.md`.
- `SparqlDidHandler::sign_with_did` (`sparql_did.rs`) intentionally **fails closed** (returns `Err`) — the SPARQL query layer holds no private keys. Sign via the identity/key-vault layer instead. It previously returned a forged all-zero 64-byte signature.

---

## 9. Per-step progress logging (PROJECT RULE)

When executing a multi-step plan/sequence (e.g. the STELLAR §A phases A0→A7), **append a dated
entry to a single, well-named progress-log `.md` at the end of every step — before starting the
next one.** This log is how the human (Timothy) sees results and decides where to help; it is not
optional.

Each entry must contain, plainly and honestly:
1. **Step / phase** + status (done / partial / blocked).
2. **What was built** — files touched, the mechanism in one or two sentences.
3. **Measured results** — real numbers (or "not measured"); never extrapolate a kernel figure to
   end-to-end. State the caveats (what the number does and does not mean).
4. **⚑ Where I need the human** — the curation-grade / out-of-band items only Timothy can decide
   or supply (eval corpus, attested content, acceptable-quality threshold, direction calls), framed
   as concrete asks so he can act. If none, say "none this step."
5. **Next step** + any new follow-ups discovered.

Logs are honest engineering records (errors and regressions included), mirror the measurement-honesty
rule, and never contain personal circumstances. The active log for the perf push is
[`STELLAR_A_PROGRESS_LOG.md`](STELLAR_A_PROGRESS_LOG.md); start a new log per major workstream with a
descriptive name.

---

## 10. Multi-agent collaboration — announce before you act (PROJECT RULE)

More than one instrument works this repo at once (separate worktrees/branches), **all allocated by
Timothy**. Before writing ANY code, every instrument — *including the LLM-lane Claude instance* — must:

1. **Read the allocation + the live feed.** [`WORK_ALLOCATION_PLAN.md`](WORK_ALLOCATION_PLAN.md) says
   who is allocated what, plus the off-limits lists (§0.2). `coordination/NOTICES.md` (canonical
   absolute path `C:\Projects\qualiaDB\coordination\NOTICES.md`, shared across all worktrees) is the
   live feed of what each instrument is touching right now.
2. **Check for collision, then defer — do not compete.** If the files you intend to touch are another
   instrument's allocation/off-limits, or already `CLAIM`ed in `NOTICES.md`, **stop. Do not start, do
   not duplicate, do not "reconcile" their work against yours.** You have no lane to defend and no
   territory to fortify. Report it to Timothy and await his (re)allocation — he disposes.
3. **Announce.** Append a dated one-line notice to `NOTICES.md` on `CLAIM` (start), `PROGRESS`
   (milestone), `BLOCKED`, and `RELEASE` (done/handed back). That is how the other instruments — and
   Timothy — see your progress without re-deriving it and burning his tokens.

The full protocol (notice format, anti-competition rules, who arbitrates) is
[`WORK_ALLOCATION_PLAN.md`](WORK_ALLOCATION_PLAN.md) §6.

## 11. Big file → library with a sub-directory (PROJECT RULE — Timothy, 2026-06-25)

**When a source file is going to become big, make it a library with a sub-directory** — do not let a
single `.rs` keep growing. Convert `foo.rs` into `foo/mod.rs` plus focused submodules
(`foo/<concern>.rs`), each a cohesive unit with its own tests. The public module path
(`crate::…::foo::*`) is preserved by `mod.rs`, so this is a safe, non-breaking refactor.

- **Threshold:** if a file is heading past ~400–500 lines, or already mixes several distinct concerns,
  split it. Prefer doing this *as* you add the code, not after it sprawls.
- **Each submodule owns its `#[cfg(test)]`.** `mod.rs` re-exports the public surface and wires submodules.
- Applies to every crate. This keeps modules reviewable and keeps "is this library complete?" answerable
  per-concern instead of by scrolling one 1500-line file.
- **Refinement (Timothy, 2026-06-25):** the PRIORITY is full implementation **without creating new
  monolithic files** — split *as you go* for the modality libraries you are actively building out (e.g.
  `abductive/`, `argumentation/`). **Pre-existing monoliths** (e.g. `deontic.rs`, `graph_theory.rs`) and any
  split that would be a risky refactor mid-feature are **deferred to a dedicated "library-ization" pass run
  once everything is working** — that pass also resolves items that couldn't be done otherwise. Do not
  block a full implementation on a split; flag the file and move on.

## 12. Completeness bar (PROJECT RULE — Timothy, 2026-06-25)

**Fully implement. Do not skip "hard" progress and dress the gap up as an honest follow-up.** The
acceptance test for any library/module is: *an independent reviewer asked "is this complete?" answers
**yes***. A `// TODO`, an `⚑ honest follow-up`, or a `◑ partial` left in place of real work is a
**failure of the task**, not a virtue. If something genuinely cannot be done by the agent (it needs an
out-of-band decision or datum only Timothy can supply — e.g. sensitive vocabulary he reserves the right
to coin), that is the *only* allowed reason to defer, and it must be surfaced as a single crisp ask, not
buried. Measurement honesty (don't claim done when it isn't) and this completeness bar are the same
coin: say what's true, and make what's true be "done."

## 13. Modernize to current dependency APIs + fix problems along the way (PROJECT RULE — Timothy, 2026-06-26)

When you touch code that uses a **stale or deprecated dependency API**, bring it up to the **current
version's API and capabilities** — do not add a workaround, keep an old pattern, or step over the
breakage. Dependencies are bumped deliberately; the methods that call them must be updated to the new
surface, and should **adopt the better capabilities the new version offers**, not merely be made to compile.

- **Concrete live example:** `wgpu` is pinned to **29** (`crates/qualia-core-db/Cargo.toml`), but some
  GPU code still calls the old ~0.20 surface (e.g. `wgpu::Maintain`, replaced by `PollType`) — that
  mismatch is currently breaking the build. Update such call sites to wgpu 29's API + capabilities. **The
  rule is not wgpu-specific; it applies to every dependency** (the example is just the one in front of us).
- **Fix problems you hit along the way**, within your allocated scope, tested and behaviour-checked —
  don't leave adjacent breakage silently, and don't dress a real fix up as out-of-scope.
- **This never overrides the lane rules (§10).** If the stale-API code is in another instrument's live or
  off-limits lane, **flag it** (a `NOTICES.md` line + report to Timothy) rather than reaching in.
  Fix-along-the-way is not a licence to barge.
