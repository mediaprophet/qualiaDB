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
1. `gguf_sharder.rs` — parses GGUF header, generates `QualiaQuin` pointer map (byte offsets encoded into quin object field, upper 4 bits = modality flag `0b1001`)
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
3. `validate_output(output)` — post-flight. Output must have ≥1 provenance `QualiaQuin` citation. Ungrounded output is rejected.

This is **mandatory infrastructure**, not optional middleware. Do not bypass or stub it.

---

## 5. The Daemon on Port 4242 Is the Graph Engine

```
localhost:4242  =  Qualia semantic graph daemon
                   Endpoints: /health, /query (SPARQL-style over quins)
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
| 48-byte `QualiaQuin` for all semantic data | Everything is bit-packed into 6 × `u64` fields |
| 42 MB `SlgArena` ceiling | The Webizen VM must fit within this; allocating past it is an OOM |
| `q_hash()` for all URIs | No runtime string allocation; FNV-1a at compile time |
| Opcodes `0x10+` for new modalities | `mini_parser.rs` owns `0x00–0x04`; never overlap |

---

## 7. What to Read First for Common Tasks

| Task | Start here |
|------|-----------|
| Modifying inference | `llm_agent.rs`, then `gguf_bridge.rs` |
| Adding a logic modality | `AGENTS.md §3` + `deontic_logic.rs` as template |
| Touching the graph engine | `orchestrator.rs`, `storage.rs`, `wal.rs` |
| MCP server changes | `mcp_server.rs` |
| Benchmark harness | `benchmarks/harness.py`, `benchmarks/qualia/runner.py` |
| Governance / rights | `webizen.rs`, `agency.rs`, `deontic_logic.rs` |
| Profile / identity | `profiles.rs`, `key_vault.rs`, `identifier.rs` |
| Scientific primitives | `webizen.rs::execute_vm_frame` (fully wired, not stubs) |

---

## 8. Known Inaccuracies to Watch For

- `ARCHITECTURE.md §5` previously said "llama.cpp" — **corrected** 2026-06-06. The backend is `wgpu`, not llama.cpp.
- `logic.rs::Always/Eventually/Next` opcodes are **not** real LTL operators — they compare a float threshold on a single Quin. Use `temporal_ltl.rs::evaluate_ltl_trace` instead. See `AGENTS.md §4-B`.
- `logic.rs::extract_float` uses `0b001 << 60` as an f32 tag, conflicting with `resolver.rs` which uses the same bits for `xsd:integer`. See `AGENTS.md §4-D`. Do not "fix" this unilaterally.
