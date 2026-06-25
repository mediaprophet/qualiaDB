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

## B. Hard-invariant conflicts — cannot be done without changing a core invariant

| Item | Conflict | Substitute / needed decision |
|------|----------|------------------------------|
| **`spatio_temporal.rs` — "exact floating-point geometric intersections for RCC-8"** | A 48-byte `NQuin` **cannot carry region-boundary geometry** (AGENTS.md / CLAUDE.md §8; memory `project-logic-modalities-activation` records RCC-8 as *unwireable*). | The jurisdiction-hierarchy subsumption (`deontic_compose::obligation_applies_in`) is the encodable spatial substitute. ⚑ **Decision:** change the NQuin layout to carry geometry, or accept the substitute? (NOT YET REACHED — will flag at `spatio_temporal`.) |

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
write CPU-side logic but cannot verify GPU paths *green* like everything else. Affected (mostly NOT YET
REACHED): `calculus/cuda_bridge.rs`, `calculus/host.rs` (SIMD alignment), `calculus/tensor_provenance.rs`
(zk-SNARKs over tensors), `calculus/ode_solver.rs` (GPU paths), and the `diffusion` GPU pass (CPU side done).
⚑ **Decision:** may I use the A2000 to verify GPU items, or do them **CPU-side-only + mark the GPU path
unverified**? (Note: QPU/quantum is separately **deprioritized** by you — WAP §0.11 — and is out of scope.)

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
3. **`spatio_temporal` RCC-8** — change the NQuin layout to carry region geometry, or accept the jurisdiction-hierarchy substitute?
4. **asp CDNL solver engine** — prioritize as its own project, or leave the capability-complete `[~]`?
