# Production-Excellence Audit — Boundary Deferrals

**Purpose.** This is the honest, single-page record of everything in
[`audit_production_excellence_tasks.md`](.dev-docs/to-do/audit_production_excellence_tasks.md) that is
**intentionally NOT marked fully done**, and exactly *why*. It exists so nothing is silently incomplete —
each item here is a genuine boundary (a hard invariant, a multi-year engineering scope, a human decision,
another instrument's allocation, or the deferred library-ization pass), **not** a skipped "hard bit."

Companion to [`AUDIT_PRODUCTION_EXCELLENCE_PROGRESS_LOG.md`](AUDIT_PRODUCTION_EXCELLENCE_PROGRESS_LOG.md)
(the per-step results record). Branch: `0.0.20-production-excellence`. Living doc — appended as boundaries
are reached.

---

## A. Capability boundaries — real engineering scope beyond a single full-impl pass

| Item | Status | Where | Audit mark |
|------|--------|-------|-----------|
| **`asp.rs` — Clingo/clasp CDNL *performance* parity** | ASP *capability* is COMPLETE (stable models via Gelfond-Lifschitz, weak-constraint optimization, grounder, cautious/brave, paraconsistent routing). Literal clasp-grade conflict-driven-nogood-learning solver-engine *performance* is a separate multi-year systems effort. | `7307de864` | `[~]` |
| **`manifold_logic.rs` — CudaGPUDirect 10D-tensor manifold *renderer*** | CPU topological-data-analysis is COMPLETE (Vietoris-Rips `vietoris_rips_b0`, persistent H0 `persistent_h0`, dimension bridging). The GPU 10D renderer is the module's own *documented* separate effort (STELLAR #11–13, `compute_universe.rs`). Also gated by §E. | `546c56df1` | `[~]` |
| **`consensus.rs` — full multi-phase BFT *protocol***  | The BFT *safety logic* (quorum/`3f+1`, equivocation, light-client gate) is done & checked. A full networked PBFT/HotStuff state machine (view-change, pipelining) is a separate systems build — noted inline on the checked item. | `e8cfa3e7c` | `[x]` + note |
| **`dl.rs` — full ALC/SROIQ model-construction *tableau*** | The STRUCTURAL constructs are done & checked (concept disjointness + clash detection, role hierarchy + transitivity, qualified cardinality, nominals; plus subsumption). The full ALC/SROIQ TABLEAU (∃/∀ expansion with individual generation + blocking — what HermiT/Pellet do) is research-grade AND conflicts with the zero-heap invariant (a tableau builds a *dynamic* model tree, not bounded fixed arrays). | (dl batch) | `[~]` ×2 |
| **`probabilistic.rs` — junction-tree/clique-tree** | Exact inference is done via variable *enumeration* (`update_beliefs`, zero-heap, bounded). Junction-tree/clique-tree is the large-network *efficiency* variant of the same exact inference — a perf optimization, not a missing capability. | `a0f13296e` | `[x]` + note |
| **`ctl.rs` — CTL\***  | The full CTL operator set (EX/AX/EF/AF/EG/AG/EU/AU) + Emerson-Clarke labelling + fairness is done & checked. CTL\* (arbitrary nesting of path quantifiers and temporal operators) is a strictly more expressive logic — a separate, harder model-checking problem. | `7e9715b90` | `[x]` + note |
| **`tensor_provenance.rs` — FHE over ODEs** | The append-only tamper-evident lineage DAG (`tensor_integrity.rs`, BLAKE3 content-addressed) and the zk binding for linear transformations (real Groth16 `private_matrix_multiply`) are DONE. **Homomorphic-encryption evaluation of a differential-equation integrator over ciphertexts** has no in-tree backend (no `tfhe`/`concrete`/`seal`), and FHE across the multiplicative depth of an RK4 loop is a multi-year crypto-systems effort. Adding a heavyweight FHE dep also conflicts with the affordability/honest-scope rule. | `6e53ed467` | `[~]` |
| **`tensor_provenance.rs` — general per-op zk-SNARK** | zk for the stated goal (linear `y=W·x`, hide `W`) is the REAL Groth16 `private_matrix_multiply`, bound via `transformation_commitment`. A zk-SNARK over *arbitrary non-linear* tensor ops needs a bespoke R1CS circuit per operation — research-scale generalization beyond the soundness-tested linear case. | `6e53ed467` | `[x]` (linear) + boundary (general) |

## B. Hard-invariant conflicts — cannot be done without changing a core invariant

**RESOLVED (2026-06-25): RCC-8 is NOT a boundary.** On reaching `spatio_temporal.rs`, verification showed
`evaluate_rcc8_points` already does **exact floating-point RCC-8** (ray-cast point-in-polygon + boundary
collinearity tests, all 8 relations) over a **bounded vertex *slice*** — the geometry rides in a caller-supplied
slice, NOT inside one 48-byte NQuin, so the "unwireable" memory was stale. No invariant change needed. (No
items currently in this section.)

## C. Human-input boundaries — need Timothy's decision/datum (the CLAUDE.md §11 exception)

| Item | What's done | What's needed |
|------|-------------|---------------|
| **`capacity.rs` — guardianship domain taxonomy VOCABULARY** | The *mechanism* is COMPLETE: selective + attenuating + revocable + chained delegation (`guardianship_authorized`, `effective_principal_scoped`, `delegation_attenuates`, `authorized_after_revocation`, `chain_authorizes`). `9b1e52437` / `4345d284a`. | ⚑ The named 17-domain set is `©CopyOfGuardianShipRelations` (private, must-not-touch per WAP §0.4). **Decide: (A)** give me your canonical domain list, or **(B)** approve binding a standard legal taxonomy (Medical/Financial/Legal/Healthcare/Residential/Educational/Reputational/End-of-Life/AI-Proxy…) as a renamable placeholder. |

## D. Allocation boundaries — another instrument's work

| Item | Why |
|------|-----|
| **`n3_parser.rs` (4 items)** | Allocated to the separate active **`qualia-n3-parser` worktree** (WORK_ALLOCATION_PLAN + NOTICES). Left to that worktree; not mine to touch. |

## E. GPU verification boundary — pending Timothy's GPU-test decision

The A2000 is reserved for the LLM lane and the standing rule (WAP §0.10) is **never run GPU tests**. I can
write CPU-side logic but cannot verify GPU paths *green* like everything else. Affected, STILL BLOCKED:
`calculus/cuda_bridge.rs` (GPUDirect/DirectStorage bridge) and `calculus/host.rs` (SIMD alignment /
Smith-Waterman SIMD). The `diffusion` GPU pass has its CPU side done.

**Resolved CPU-side (no longer GPU-blocked):** `calculus/ode_solver.rs` advanced integrators are done in
`ode_advanced.rs` (pure-scalar, no GPU) — `6e53ed467`; `calculus/tensor_provenance.rs` zk + lineage are done
in `tensor_integrity.rs` (BLAKE3 + CPU Groth16, no GPU) — `6e53ed467`.

⚑ **Decision still needed:** may I use the A2000 to verify the remaining `cuda_bridge.rs` / `host.rs` GPU/SIMD
items, or do them **CPU-side-only + mark the GPU path unverified**? (Note: QPU/quantum is separately
**deprioritized** by you — WAP §0.11 — and is out of scope.)

## F. Deferred structural work — the "library-ization" pass (you directed this)

Pre-existing files over the ~450-line split threshold are deferred to a **dedicated library-ization pass run
AFTER all functionality works** (you directed: split-as-you-go for files I build out, but defer pre-existing
monoliths). Queue: `logic/deontic.rs` (~1380), `graph_theory.rs` (930), and others as encountered. Also: the
full **zero-heap rewrite** of the Dung `argumentation/` library (currently off-hot-path heap; the hot-path
primitive `grounded_contains` is already zero-heap) — see progress log §8.

---

## ⚑ Consolidated open questions for Timothy

1. **Guardianship vocabulary** — (A) your canonical domain list, or (B) approve a standard renamable placeholder?
2. **GPU tests** — may I verify GPU `calculus/` items on the A2000, or CPU-side-only + mark GPU paths unverified?
3. **asp CDNL solver engine** — prioritize as its own project, or leave the capability-complete `[~]`?

   _(Resolved 2026-06-25: the spatio_temporal RCC-8 question is moot — `evaluate_rcc8_points` already does exact float RCC-8 over a bounded vertex slice. No NQuin-layout change needed.)_
