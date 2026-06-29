# DAG-IR Forge — a compute-graph IR for deterministic, multi-backend kernel generation

**Status:** scoped (2026-06-29), Phase 1 in progress.
**Supersedes the codegen half of** [`deterministic-wgsl-forge.md`](deterministic-wgsl-forge.md):
that document's per-`BuiltinKernel` emitters remain valid and certified; this plan generalizes
*how* kernels are expressed and lowered so the generator stops accreting per-`kernel.id`
string emitters and instead lowers a typed **compute DAG** to every backend in one pass.

This design was produced by a parallel research + judged-design + adversarial-verification pass
against the real tree (workflow `dag-ir-forge-design`, 10 agents). The two problems that pass
surfaced are recorded as **Known constraints** below, not smoothed over.

---

## 1. Why — the limit we are removing

The forge generates kernels, but its codegen grew as a monolith: `emit/wgsl.rs::emit_kernel_body`
is an `if kernel.id == "..."` chain, and the heavier kernels (`gemm`, `gemv`, `fft`,
`ternary-gemv`) are **hand-written WGSL functions** with, at most, a separate CUDA-C f64 string.
**MSL/HLSL/PTX emit zero of them** (verified: grep count 0). So:

- New compute patterns (physics, fluid, and crucially the **LLM kernels** — fused attention,
  softmax, RMSNorm, block-dequant) each require a new hand-written per-target string. That does
  not scale and does not fulfil the forge's mandate: *produce these kernels, multi-backend,
  without an LLM.*
- The substrate to fix it already exists. QualiaDB's **q42 format natively represents DAGs**
  (Merkle ancestry in `git_bridge.rs`, causal `causeOf` edges in `causal.rs`, `did:q42`
  topological byte-offset pointers, an in-memory `DagStore` with topological traversal). A
  **compute DAG** — op-nodes + data-dependency edges, topologically ordered — is the same
  structure. The forge should lower *that*, not a flat scalar spec.

The LLM payoff is the same investment: a transformer decode step **is** a compute DAG —
`embed → (RMSNorm, attention[matmul·softmax·matmul], SwiGLU-FFN[gather-dequant·matmul·elementwise])×L → logits`.
Every node is one of a small op vocabulary that a fluid/physics kernel also draws from. One IR,
one lowering, both goals.

---

## 2. Op-node vocabulary (≤12 classes)

Tensor-level op-*classes* (not scalar ops). Each declares which hardware unit backs it; the
**type system enforces the split** (e.g. `Neighbor` cannot carry a tensor-core flag, `MatMul`
cannot carry a radius).

| Node | Signature (in → out) | Backs onto | Priority | Role |
|---|---|---|---|---|
| **Elementwise** | `T×N → T`; fn ∈ {Mul,Add,Fma,Scale,Bias,Silu,Gelu,Relu,Exp,RecipSqrt,Recip} | ALU/FMA | seed | SwiGLU `silu(g)·u`, RMSNorm `x·rsqrt`, softmax `exp(x−max)`, RoPE, affine |
| **MatMul** | `[M,K]·[K,N]→[M,N]`; `tc`, `trans_b` | tensor-core (WMMA) / CPU floor | seed | QKVO proj, `QKᵀ`, `AV`, FFN up/gate/down, logits. `M=1` ⇒ GEMV |
| **Fft** | `[L] complex → [L]`; `len` pow2, `inverse` | ALU | seed | audio/transforms (lifted byte-equal); not on LLM hot path |
| **Reduce** | `[…,A,…] → […,1,…]`; op ∈ {Sum,Max,Mean,L2}, axis | shared-mem tree + Barrier | next | RMSNorm variance, softmax max/denominator |
| **GatherDequant** | `table[V,D] packed, idx[T] → [T,D] f32`; scheme ∈ {Q4_K,Q8_0,Ternary,F16} | gather | next | embed lookup + per-block weight dequant; fuses into MatMul |
| **Broadcast** | `T[s], shape → T[shape]` | index remap | next | RMSNorm scale fanout, mask add, bias fanout |
| **Softmax** *(sugar)* | `[…,A] → […,A]` | reduce | next | legalizes to Reduce→Broadcast→Elementwise; never a leaf emitter |
| **Stencil** | `grid, taps → grid`; kind ∈ {Laplacian,Divergence,Advection,RopePair} | ALU | later | fluid Poisson/advection; RoPE as a 2-tap stencil |
| **ScatterAccum** | `src[P,…], idx[P,K] → accumulated`; op ∈ {Add,Max} | atomic add | later | SPH density/force, N-body short-range |
| **Neighbor** | `points[N,D], query[Q,D] → shortlist[Q,Kmax]`; kind ∈ {Frnn,Knn,Range}, dims, encoding | **RT-core** / grid CPU | later | SPH/N-body cutoff, k-NN, ANN candidate-gen |

---

## 3. IR types (`wgsl_forge/ir/graph.rs`)

Arena-backed, fixed-size, **no `Vec`/`String` in the hot path** (the core ABI rule). Acyclic by
construction; topological order == insertion order; `build` asserts acyclicity + shape
compatibility.

```rust
#[repr(transparent)] pub struct NodeId(u32);     // arena index
#[repr(transparent)] pub struct TensorId(u32);
pub struct Shape { pub dims: [u32;4], pub rank: u8 }      // fixed-rank ≤4; no Vec
pub enum DType { F32, F16, U32, Q4K, Q8_0, Ternary }
pub struct TensorRef { producer: NodeId, tensor: TensorId, shape: Shape, dtype: DType, layout: Layout }

pub enum OpNode {            // op-CLASS payload; ≤12 arms
  Elementwise{ f: EwKind }, MatMul{ m:u32,n:u32,k:u32, tc:bool, trans_b:bool },
  Fft{ len:u32, inverse:bool }, Reduce{ op:RedKind, axis:Axis },
  GatherDequant{ scheme:DType, block:u32 }, Broadcast{ shape:Shape },
  Softmax{ axis:Axis }, Stencil{ kind:StencilKind, halo:u32, axis:Axis },
  ScatterAccum{ op:AccumKind }, Neighbor{ kind:NbKind, k_or_r:f32, dims:u8, enc:NeighborEnc },
}
pub struct GraphNode { op: OpNode, ins:[TensorRef; 4], n_in:u8, out: TensorRef, sched: Schedule }
pub struct ComputeGraph { nodes: ArrayVec<GraphNode, MAX_NODES>, outputs: ArrayVec<NodeId, MAX_OUT> }

impl KernelSpec { pub fn to_graph(&self) -> Result<ComputeGraph, ForgeError>; }  // 1-node bridge
```

---

## 4. Lowering — one pass, replacing the `kernel.id` monolith

A visitor `trait Lowerer` with one method per op-class, plus
`fn lower_graph<L: Lowerer>(g: &ComputeGraph, l: &mut L)` that walks nodes in topological
(insertion) order and dispatches on `OpNode`. Each backend implements `Lowerer` **once**:
`WgslLowerer`, `CudaCLowerer`, `MslLowerer`, `HlslLowerer`. SPIR-V stays WGSL→naga `spv-out`;
PTX keeps its hand-written affine path (+ `Err("unsupported")` for the rest), unchanged.

**Integration point:** `emit/wgsl.rs::emit_kernel_body` (the `kernel.id` chain) and
`emit/mod.rs::emit_shader`.

---

## 5. q42 substrate binding (honest: a NEW use, deferred to Phase 6)

The q42 DAG substrate **exists and is reusable but was not built for data-flow** — it carries
version-control Merkle ancestry (`git_bridge.rs` `dag:parent`) and causal reasoning
(`causal.rs` `causeOf`), neither of which is compute data-flow.

- **Reused as-is:** the 48-byte `NQuin` record, the page-aligned memmapped superblock, the
  in-memory `DagStore` (topological traversal, Lamport snapshots), `did:q42` topological pointers.
- **Must be added (none exists):** a distinct data-flow edge predicate `q_hash("q42:feedsInto")`
  (so compute edges are not confused with VC/causal edges); an op-kind modality opcode in the
  **0x10+ range** (`mini_parser` owns `0x00–0x04` — no overlap, per core invariant §6); companion
  quins `(node, q42:tensorShape, …)`, `(node, q42:dtype, …)`, `(node, q42:scheduleHint, …)` with
  shape/hint **bit-packed into object `u64` fields** (zero-copy).

**The in-arena `ComputeGraph` is the source of truth**; the quin encoding is a
serialization/persistence + Merkle-provenance *view*, wired only once the in-memory path
(Phases 1–5) is green. It is **not on the Phase-1 critical path.**

---

## 6. RT cores — exactly one op-class, with a hard boundary

RT cores are a 3-D fixed-function BVH-traversal + AABB/triangle-intersection engine. They back
**exactly the `Neighbor` op-class** (fixed-radius / kNN / range proximity) and **nothing else**.

- **Reuse path (no new RT plumbing):** the already-certified `Op::Intrinsic(Intrinsic::RayQuery)`
  / `RayProbe` kernel (naga-validated, GPU-certified on the A2000). The only new code is a
  `build_aabb_scene()` beside `execute/wgpu.rs::build_triangle_scene` (point→bounding-sphere AABB
  BLAS) and an AABB `BufferElement` variant; the dispatch/bind protocol already exists.
- **Honest boundary (stated so uplift expectations are real):** RT cores accelerate **zero dense
  linear algebra** — no GEMM, GEMV, FFT, attention, softmax, or dequant matmul. Those are the LLM
  forward pass and belong to **tensor cores** (`MatMul.tc=true → CUDA WMMA`) and shader FMA.
  `Neighbor` is **broad-phase / candidate-generation only** — it returns a shortlist; the exact
  metric is a downstream `MatMul + Reduce` rerank. It is gated on a declared 3-D encoding: `dims>3`
  without a faithful projection makes `legalize()` refuse RT and fall back to the grid/all-pairs
  CPU path. Its differential oracle vs an exact grid is **mandatory** (RT proximity is
  approximate). RT's only contact with the LLM stack is optional **ANN candidate-gen** for
  RAG/KV retrieval — never the matmul.

---

## 7. Certify — reuse the existing machinery per node

In the seed phases **one node == one kernel**, so the current evaluators already cover it. A
whole-graph certificate = **AND of per-node certificates**; a stale node re-certifies alone.

- **CPU oracle:** each `OpNode` has a deterministic CPU floor (Phase 1 reuses the existing
  `gemm_cpu`/`gemv_cpu`/`dft_cpu`/`affine` verbatim; later phases add `reduce_cpu`/`dequant_cpu`/
  `frnn_grid_cpu`/`stencil_cpu`/`scatter_cpu`). The **whole-graph oracle composes node oracles in
  topological order**, threading each node's CPU output into the next — the graph is graded against
  a CPU interpretation of the same DAG.
- **GPU certify:** per node via `oracle::evaluate_builtin` / the generic `OracleContext`, compared
  with `compare_f32` under the node's tolerance, stored via `ManifestCache::store_certification`
  with the existing blake3 key `(adapter, semantic_hash, schedule, tolerance)`.

---

## 8. Known constraints (from adversarial verification — recorded, not glossed)

1. **Multi-node GPU execution has per-node submit latency.** `WgpuPipeline::dispatch()` builds and
   submits **one encoder per call** (`execute/wgpu.rs`). The Phase-4 graph executor (Option A) loops
   `dispatch()` per node with intermediates kept GPU-side in `QualiaSlabAllocator` — correct, but it
   pays a `queue.submit()` per node. The single-encoder/deferred-submit multi-pass fusion (Option B)
   is an **explicit later perf pass**, not claimed now. Intermediate tensors do stay GPU-side (no CPU
   readback mid-graph); only the submit is per-node.
2. **Phase-1 byte-equality is load-bearing for cache validity.** The certify cache key includes a
   `source_hash`; any incidental reformatting of emitted WGSL silently invalidates cached
   certificates. **Mitigation (adopted):** in Phase 1 the `WgslLowerer` leaf methods **delegate to
   the existing, proven `emit_{gemm,gemv,fft,affine}_wgsl` functions** rather than re-typing them, so
   the emitted bytes are identical *by construction*. A test asserts graph-path output ==
   direct-legacy output for the 4 routed ids; the existing certify/tune suites must stay green. Native
   from-scratch graph templates replace the delegations incrementally in Phases 3+, each behind the
   same byte/numeric-equal gate.
3. **`Neighbor` grid fallback for `dims>3` is not yet enforced in code** — it is a Phase-7
   `legalize()` requirement; until then `Neighbor` is unavailable, not silently wrong.

---

## 9. Phases (each an independently verifiable deliverable)

| # | Title | Deliverable | Verification |
|---|---|---|---|
| **P1** | **Minimal spine + byte-equal WGSL slice** | `ir/graph.rs` (NodeId/TensorRef/Shape/DType/OpNode{Elementwise,MatMul,Fft}/GraphNode/ComputeGraph, arena, acyclic+shape asserts, unit-tested). `Lowerer` trait + `lower_graph` + `WgslLowerer` **delegating** to the existing emitters. `KernelSpec::to_graph()` for `{gemm,gemv,fft,affine-f32}`; `emit_kernel_body` routes **only** those 4 through `to_graph→lower_graph`; the other 5 ids keep their existing branch. | Byte-equal test (graph-path == legacy) for the 4 ids; the existing gemm/gemv/fft/affine certify+tune suites stay **green & byte-identical** on the A2000; `cargo test` + naga-validate pass. |
| **P2** | **Reduce + Broadcast (native nodes)** — ✅ DONE | `wgsl_forge/graph_ops/{reduce,broadcast}.rs`: native WGSL templates emitted **directly from the graph node** (the first non-delegating lowerings) via `WgslGraphLowerer` + `emit_graph_wgsl`; exact CPU oracles; the RMSNorm/softmax primitives | **GPU-certified on A2000**: `reduce_gpu_matches_oracle` (Sum/Mean/Max/L2) + `broadcast_gpu_matches_oracle` pass; naga-validate + CPU hand-checks green |
| **P2b** | GatherDequant + ternary `{GatherDequant→MatMul}` template | block-dequant gather (Q4_K/Q8_0/Ternary) reusing the certified ternary/dequant path | **moved to land with the P4 multi-node executor** — the ternary split is a *2-node* graph whose intermediate must stay GPU-side, which needs the topo-order multi-dispatch P4 builds (verifier finding §8.1). Not half-built here. |
| **P3a** | **Elementwise LLM kit (native nodes)** — ✅ DONE | `graph_ops/elementwise.rs`: unary `Silu/Gelu/Exp/RecipSqrt/Relu/Recip`, binary `Add/Mul`, ternary `Fma` — emitted from `OpNode::Elementwise`; exact CPU oracles. (`Scale`/`Bias` defer to the affine kernel, explicit `Err`.) | **GPU-certified on A2000**: `elementwise_gpu_matches_oracle` passes (all activations + Add/Mul); naga-validate each arity + CPU hand-checks green |
| **P3b** | Softmax sugar + `MatMul→Elementwise` fusion (fused-ffn emergent) | Softmax legalizes to `Reduce→Broadcast→Elementwise` (the nodes now all exist); fusion rewrite reproduces fused-ffn | **multi-node** graphs → GPU-certify lands with the **P4 executor**; the CPU-composed oracle is buildable independently. Not faked ahead of the executor. |
| **P4** | Full LLM decode DAG + composed oracle | express `embed→[RMSNorm, attn, SwiGLU]×L→logits` in op-nodes; whole-graph CPU oracle (compose in topo order); topo-order GPU dispatch (Option A) | one block graded vs composed CPU floor on A2000; **honest kernel-level uplift** reported (no end-to-end extrapolation) |
| **P5** | Second backend in one pass: `CudaCLowerer` | implement `CudaCLowerer` for seed+LLM nodes (`MatMul.tc → WMMA`), reusing `OracleContext` so the **same graph** lowers to WGSL and CUDA-C with no per-id branches | CUDA-C cross-backend differential oracle green on A2000; same graph certifies on both backends |
| **P6** | q42 substrate binding | `q42:feedsInto` predicate, op-kind opcode 0x10+, tensorShape/dtype/scheduleHint companion quins; serialize `ComputeGraph` ↔ NQuins; topo-sort + Merkle ancestry via `DagStore` | round-trip graph→quins→graph reproduces an identical certify; zero-copy assertion holds |
| **P7** | RT `Neighbor` + remaining backends + Stencil/Scatter | `Neighbor` lowering to the certified RayQuery path; `build_aabb_scene` + AABB `BufferElement`; `legalize()` grid fallback for `dims>3`; `Stencil`/`ScatterAccum`; `MslLowerer`/`HlslLowerer` for portable nodes | `Neighbor` **mandatory** oracle vs exact grid (recall-checked) on A2000; high-dim falls back to grid; SPH/N-body cutoff certifies; MSL/HLSL nodes naga/dxc-validate |

**Riskiest item, de-risked first (per the verifier):** Phase-1 byte-equality — addressed by the
delegation mitigation in §8.2 (the lowerer calls the proven emitters; nothing is re-typed).

---

## 10. What this does and does not change

- **Replaces** the per-`kernel.id` WGSL string sprawl with one DAG lowering — the generator gets
  *more* general as the per-target code *shrinks*. The existing `BuiltinKernel` certify identities
  and caches are preserved (byte-equal gate).
- **Unifies** non-LLM kernel production (physics/fluid) and the LLM decode pipeline under one IR.
- **Does not** make RT cores accelerate dense matmul (they back `Neighbor` only), and **does not**
  claim free multi-pass GPU fusion in Phase 4 (per-node submit; fusion is a later perf pass).
- **Does not** require q42 persistence to be useful — the in-arena graph is the source of truth;
  q42 binding (Phase 6) adds persistence + Merkle provenance, not correctness.
