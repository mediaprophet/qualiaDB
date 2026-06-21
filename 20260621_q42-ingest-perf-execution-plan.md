# `.q42` GGUF/Safetensor Ingest + Performance — Execution Plan

**Date:** 2026-06-21
**Scope:** qualiaDB repo only (the browser depends on a fully-defined qualiaDB; not in scope here).
**Focus (your stated concern):** reach the point where the advanced post-LLM `.q42` structures **ingest
GGUF/safetensors** and deliver **enhanced performant capabilities** — under no-mocks discipline, with the
MCP collaboration bridge driving agent work.
**Companions:** `20260621_QualiaReview.md` (blueprint), `LLM_Q42_STRATEGIC_PLAN.md` (lanes/native status),
`20260621_arch-enhance-status.md` (real-vs-simulated audit), `AGENT_INTENT_LOGGING_SPEC.md`.

> **Grounding posture:** this plan is written against the **verified** state of the tree (this session's
> audits), not the optimistic blueprint. Where the blueprint and the code disagree, the code wins and is
> flagged. Honesty over false hope — per the no-mocks mandate, which this plan operationalises rather than
> just restates.

---

## 0. Verified current state (what's REAL / SKELETON / ABSENT today)

| Capability | State | Evidence |
|---|---|---|
| GGUF → `.q42` **weight container** (`Q42W`) | ✅ REAL | `q42_weight.rs` (699 L): header+manifest+tokenizer+CRC, 16K-page-aligned opaque blobs, byte-parity test vs GGUF. |
| Supported quant types | ✅ REAL but **limited** | `ggml_quants.rs`: F32, F16, Q4_0, Q5_0, Q8_0, Q4_K, Q6_K only. **Q5_K/Q3_K/Q2_K/IQ\* error out.** |
| Safetensor ingest | ❌ ABSENT | only the word appears in `main.rs`; no parser. |
| `.q42` compression / down-sampling (ternary/KIVI/W4A4) | ❌ ABSENT | container stores **opaque byte-identical** GGUF blobs — "ingest Q8 → smaller/faster" is **not** true yet. |
| Semantic-graph `.q42` **volume** ingest (CLI) | ✅ REAL | `ingest/`: N-Triples/Turtle/N3/RDF-XML/JSON-LD/CBOR-LD/KML/CHK → NQuin SuperBlocks, out-of-core, 512 MB-floor-safe. **No GGUF/safetensor in `detect.rs`.** |
| Native inference | ⚠️ BROKEN at runtime | loads, then incoherent `<\|endoftext\|>` ~24 min/prompt (`LLM_Q42_STRATEGIC_PLAN.md` §A.3). |
| WASM inference | ⚠️ narrow | works in harness on SmolLM2-360M; deployed demos 404; wgpu 0.19.4 device-limit breaks real Chrome. |
| WASM64 (`memory64`) detect + OPFS fallback | ❌ ABSENT (decided) | decision recorded in `QualiaReview.md` §1; **no implementation**. |
| MCP server | ✅ REAL (~40 tools) | `mcp_server.rs` zero-heap byte-dispatch; **no backlog/pending tooling**, no `PendingImplementation` type. |
| Agent-intent logging | ⚠️ SKELETON | `agent_intent.rs`: "Simulate reading the 6 Vectors", simulated lock lease, `println!` modality stubs; N3→temporal/epistemic/defeasible dispatch is wired. |
| Simulated/scaffold modules | ⚠️ per audit | `zk_proofs` (hash-not-proof), `ambient_orchestration`, `acoustic_ble_mesh`, `csd_storage`, `ebpf_firewall`. |

**Two `.q42` families — name them, don't conflate** (the "unified archiver" idea must respect this):
- **`Q42` semantic volume** = NQuin triples + lexicon + bidx + LZ4 SuperBlocks (the knowledge graph).
- **`Q42W` weight container** = quantized tensor blobs + manifest + tokenizer (the LLM weights).
They already use distinct magics. GGUF/safetensor ingest extends the **`Q42W`** family; it should be
*surfaced through* the CLI ingest UX (one `qualia ingest` entrypoint) while writing the weight-container
format, not jammed into the triple-sorting path.

---

## 1. The governance spine: No-Mocks → `PendingImplementation` → MCP Backlog

Your correction is the cross-cutting rule for everything below: **no mocks, no fake returns, no
panicking `todo!()` in runtime paths.** Unbuilt capability is declared honestly and routed to agents.

**1.1 A typed pending state (engine-wide).** Add `qualia_core_db::PendingImplementation { task_id: u64,
constraints: &'static [&'static str] }` and a shared `QualiaError::Pending(PendingImplementation)` (today
errors are per-module — introduce one shared variant the modules can surface). Zero-alloc: `task_id =
q_hash(module_path)`, constraints are `&'static` slices.

**1.2 The Task Ledger as NQuins (not a side file).** Per the spec's "engine describes its own missing
capabilities," each unbuilt feature is a node: `⟨q42:<module>, q42:requires, q42:Implementation⟩` with a
`metadata` bit `Q42_META_PENDING`. Seed it from the **already-known** gaps (this session's audits):
`zk_proofs` (real proof vs hash-commitment), `ambient_orchestration`, `acoustic_ble_mesh`, `csd_storage`,
safetensor ingest, `.q42` compression, the `agent_intent.rs` skeleton hooks, WASM64 bootloader.

**1.3 The MCP tool `get_pending_tasks`.** Add to `stable_mcp_tools()` + a `b"get_pending_tasks"` arm in
`enforce_fiduciary_tool_dispatch`. Returns structured JSON-RPC: `{task_id, module, constraints[], status}`
read from the Task Ledger quins. A connected agent (Claude/Cursor/Antigravity) calls it to ask "what needs
building, under what invariants?" — turning tech-debt into a machine-readable, fiduciary-gated backlog.
This is the **"document + make the MCP collaboration capability function ASAP"** deliverable: §1.3 + a
`docs/manuals/standards/MCP_COLLABORATION.md` that documents the existing ~40 tools + the new backlog tool
+ the stdio handshake + the deontic firewall (`0x02` classified → `ERROR_FIDUCIARY_BOUND_VIOLATION`).

**1.4 Close the loop with intent logging.** When an agent picks up a `get_pending_tasks` item, it must
log an Intent NQuin (`OP_INTENT_LOCK`, 300s lease) per `AGENT_INTENT_LOGGING_SPEC.md` — so the backlog,
the locking, and the provenance are one system. (This also requires de-skeletoning `agent_intent.rs` — §5.)

> **Net:** §1 is small, high-leverage, and unblocks *agent-driven* completion of everything else. Build it
> first so the rest of the plan can itself be dispatched as backlog items.

---

## 2. Phase A — Foundations & invariant lockdown (prereqs, low risk)

1. **Single-source the 48-byte primitive.** Per the webizen review: the engine's `NQuin` is canonical; no
   re-definitions. (Engine already clean of UI/shell deps — keep it that way.)
2. **Feature-gate, never mock, the 25-year stubs.** Wrap unbuilt roadmap modules so production builds don't
   ship fake logic; the gated-out path returns `QualiaError::Pending` (§1.1), not a fake value. *(Note: the
   blueprint's earlier "mock-crypto feature" wording is superseded by §1 — gate to `Pending`, not to a mock.)*
3. **`get_pending_tasks` MCP tool + `MCP_COLLABORATION.md`** (§1.3).
4. **WASM64 bootloader (dual-target + detect + fallback)** — decided in `QualiaReview.md` §1, unbuilt.
   **Critical correction:** you cannot ship one "fat" binary that downgrades — pointer size (`i32` vs `i64`)
   is baked into the compiled memory instructions. So this is **two binaries + a JS switch**, not a runtime
   flag:
   - **Compile twice:** `wasm32-unknown-unknown` (stable) → `qualia_core_wasm32.wasm`; and
     `wasm64-unknown-unknown` (nightly, `-Z build-std`, Tier-3 experimental) → `qualia_core_wasm64.wasm`.
   - **JS feature-probe** (cheap, solved): `try { new WebAssembly.Memory({initial:1, maximum:1, index:'i64'}); }`
     → on `TypeError`, fall back. Then `instantiateStreaming` the chosen artifact.
   - **Wasm64 present:** map `<8B` `.q42` straight into linear memory (drastically less I/O before the GPU feed).
   - **Wasm32 only:** the **OPFS demand-paging** fallback (16K-aligned `.q42` blocks → 42 MB `SlgArena`),
     4 GB ceiling + 8 MB stack. Zero-heap invariants identical across both ("larger desk ≠ sloppy code").

   **Honest risk — the hard part is NOT the JS probe, it's whether the Rust dep tree even compiles to
   wasm64 today.** `wasm64-unknown-unknown` is Tier-3/nightly/`build-std`; `wgpu`, `wasm-bindgen`, `web-sys`,
   the three `getrandom` shims, and any crate assuming 32-bit pointers may not be wasm64-clean yet. **Action:
   spike the wasm64 build first** (does `cargo +nightly build --target wasm64-unknown-unknown -Z build-std`
   even succeed for `qualia-core-db`?) before committing CI to dual artifacts. If the engine itself
   (`#![no_std]`, fixed-width fields) compiles but a leaf dep doesn't, that dep is the blocker, not us.
   - **Cross-target ABI audit (prerequisite):** the `.q42` format + `NQuin` must be **byte-identical across
     wasm32/wasm64** — verify every serialized field is fixed-width (`u32`/`u64`), **never `usize`/pointer-
     sized**, or a wasm64 build silently changes on-disk layout. (NQuin is 6×`u64` = 48 B, pointer-agnostic;
     audit the headers/manifests around it.) This is a `get_pending_tasks` entry with that exact constraint.

---

## 3. Phase B — GGUF / Safetensor AOT ingest (your primary concern)

Goal: one `qualia ingest model.gguf|model.safetensors -o model.q42` that produces a `Q42W` container.

1. **Detect.** Add `SemanticFormat::Gguf` (magic `GGUF`) and `::Safetensors` (8-byte LE header-len +
   `{`-prefixed JSON header) to `detect.rs`; route them to a **weight-ingest** path (distinct from the
   triple path). `q42` already short-circuits "already native."
2. **GGUF path = reuse what's real.** Wire the existing `q42_weight::compile_gguf_to_q42` behind the CLI
   (it's done + byte-parity tested). Just surface it + progress + OPFS-friendly chunked write.
3. **Safetensors path = NEW, but small.** The safetensors format is a JSON header (offsets/dtypes/shapes)
   + a contiguous tensor blob — **hand-parse the header** (no inference dependency; stays within Prime
   Directive #4: a format parser is not an LLM runtime — but flag for your sign-off). Map tensors → the
   same `Q42TensorEntry` manifest + page-aligned blobs the GGUF path emits. dtype coverage: F32/F16/BF16→
   convert/representable; quantized safetensors are rare (most are F16/BF16) — decide BF16 handling.
4. **Page-aligned opaque blocks (already the design).** Keep 16K-page alignment so the native engine
   streams blocks straight to WebGPU bind groups without the global allocator (preserves the zero-copy ABI).
5. **CBOR-LD ontological header binding (neuro-symbolic seed).** During AOT, attach the reserved `.q42`
   "cold" CBOR-LD section binding tokenizer tokens → local W3C ontologies (SNOMED/FIBO/…). The header slot
   exists (`cold_offset`/`cold_len`); this fills it. Off the hot loop (parsed once at ingest).
6. **512 MB-floor discipline.** Stream in chunks (the semantic pipeline already proves the out-of-core
   pattern with `ExternalSorter`); never load a multi-GB model into the heap; `dhat-rs` must show zero
   generic-heap churn on the hot parse.

**Honest gate:** ingesting Q8/F16 today yields a *higher-fidelity but not smaller/faster* `.q42` (opaque
blobs). The size/speed win comes in Phase C. Say so to anyone expecting compression from this phase.

---

## 4. Phase C — Enhanced performant capabilities (the real lever)

This is where "advanced post-LLM structures" + performance actually land. **Currently all ABSENT.**
Sequencing matters because two things gate it:

- **C0 (gate): fix native inference + upgrade wgpu 0.19→0.20.** Native generation is broken (§0); don't
  build a compression compiler whose output you can't validate. The wgpu device-limit (`maxInterStageShader
  Components`) gates *both* the WASM LLM *and* the future WASM render path — fix once on the shared device.
- **C1 — AOT down-sampling compiler** (`q42_weight` v-next): the **Concentration-Alignment Transform**
  (AWQ-style activation scaling baked into the CBOR-LD header → W4A4 at Q8 quality); **ternary (BitNet
  1.58b)** packing of non-critical FFN layers (adds/subs instead of FMA in WGSL); keep attention at Q4.
  *Needs a calibration corpus + (for true ternary) a QAT model, not a PTQ snap — flag that honestly.*
- **C2 — KIVI asymmetric KV-cache** (2-bit keys / 4-bit values) → 100k-token context in a WGPU ring buffer.
- **C3 — Zero-copy speculative decoding** (native Swarm): `memmap2` a ~100M draft + the target; verify
  4–5 tokens/pass; zero heap on swap.
- **C4 — Native Swarm zero-copy execution**: `mmap` `.q42` SuperBlocks NVMe→VRAM via `wgpu`, layer-by-layer,
  CPU never heaps the model. (`daemon_swarm.rs` is the home; note its current sim caveats from the audit.)

Each C-item is a `get_pending_tasks` backlog entry with its invariants attached.

---

## 5. Phase D — Neuro-symbolic gateway, deontic masking, real intent logging

1. **Hardware deontic taint masking.** Wire `fused_attention.wgsl` to read the NQuin 5th/metadata vector;
   a tensor tagged `0x02` (classified) or failing a SHACL/ODRL gate is multiplied to zero **in-shader**
   before tokens emit. The `Q42_META_DEONTIC_TAINT` bit + `const ENABLE_DEONTIC_TAINT` seam already exist.
2. **Egress gatekeeper** (`daemon_swarm.rs`): inspect `webizen:SensitivityLabel` before any `nym_adapter`/
   WebTorrent send; `0x02` → severed. (Blueprint §4/§6 — currently unenforced.)
3. **De-skeleton `agent_intent.rs`** → makes §1.4 real: parse the actual 6-Vector semantic header (Who/
   When/Why/What/Where/Cost), implement real `OP_INTENT_LOCK`/`OP_NAMESPACE_LOCK` + the temporal lease
   (replace the `// Simulated` lease), and replace the `println!` modality "skeleton hooks" with either real
   dispatch or an honest `Pending` (no fake "Dispatching…" lines that do nothing).

---

## 6. Sequence (dependency-ordered)

1. **§1 governance spine** (PendingImplementation + Task Ledger + `get_pending_tasks` + `MCP_COLLABORATION.md`) — unblocks agent-driven execution of the rest.
2. **§2 Phase A** foundations (single-source NQuin, feature-gate→Pending, WASM64 bootloader).
3. **§3 Phase B** GGUF (surface existing) + Safetensor (new) ingest + CBOR-LD header.
4. **C0 gate**: fix native inference + wgpu 0.20 upgrade.
5. **§4 Phase C** compression/perf (AWQ/ternary → KIVI → spec-decode → Swarm mmap).
6. **§5 Phase D** deontic masking + egress gatekeeper + real intent logging.

**Invariants binding every step:** 48-byte NQuin ABI; zero generic-heap in hot paths (`dhat-rs` gate);
512 MB RAM floor; 8 MB WASM stack; native/wasm `cfg` isolation; engine compiles for CLI **and**
`wasm32-unknown-unknown` with no UI/windowing deps; every agent change logged as an Intent NQuin.

---

## 7. What I did NOT do / open decisions for you

- **I did not start coding** — this is the plan you asked for. Say which slice to execute first (my
  recommendation: §1, the MCP backlog spine — small, and it makes the rest dispatchable).
- **Safetensors parser dependency:** hand-roll the header parse (recommended, no LLM-lib risk) vs the
  `safetensors` crate — your call given Prime Directive #4.
- **BF16 handling** on safetensor ingest (convert to F16, or add a BF16 dequant path).
- **Coverage caveat:** I read the blueprint, intent spec, README, and the full `ingest/` pipeline +
  `agent_intent.rs` in full; I **structurally surveyed** (not line-by-line) the two large MCP files
  (`mcp_server.rs` 1311, `mcp_tool_impls.rs` 1266) and `gguf_sharder.rs` (1610). If you want the
  `get_pending_tasks` wiring authored against the exact dispatch internals, I'll read `mcp_server.rs` end
  to end first.
