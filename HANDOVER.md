# HANDOVER — Production-Excellence Audit + RDF Reasoning Roadmap

**Written 2026-06-25 for a context refresh. Read this FIRST, then resume.** Directed by Timothy.

> **⚠ PARTIALLY SUPERSEDED (2026-06-25, later same day).** This snapshot predates a large
> follow-on session. For the CURRENT state read `AUDIT_PRODUCTION_EXCELLENCE_PROGRESS_LOG.md`
> **§10–§12** first. Key deltas since this was written: the **MODALITIES section is now COMPLETE**
> (graph_theory PageRank/subgraph-iso, OWL 2 RL reasoner, identity/sovereignty SHACL, advanced ODE
> integrators, tensor lineage integrity, bioinformatics UPGMA all landed); and **`cuda_bridge.rs`
> was REMOVED** — CUDA dropped by design, capabilities folded into the vendor-neutral wgpu stack
> (`calculus/hetero_dispatch.rs`), so the "GPU/Linux hardware" boundary mentioned below is **dissolved,
> not pending**. The only MODALITIES file left is `n3_parser.rs` (another worktree's lane). Next section:
> DOMAINS.

---

## 0. TL;DR — resume here

- **Task:** fully implement every item in `audit_production_excellence_tasks.md`, with **real tested code**,
  then check it off **with the commit id**. Then build the **RDF reasoning modalities** (`RDF_REASONING_ROADMAP.md`).
- **Worktree:** `C:\Projects\qualiaDB\.worktrees\qualia-prod-excellence` · **branch** `0.0.20-production-excellence`
  (off `0.0.20`). Main checkout is untouched. **HEAD at handover:** `b6d961f56`. **35 commits** on the branch.
- **Progress (whole audit):** **147 done · 4 `[~]` boundary · 95 remaining.** MODALITIES section is ~78% done;
  the DOMAINS/SOLVERS/SPECIALIZED_LIBS/OBFUSCATION sections are barely started **and are mostly boilerplate-
  mismatched or QPU-deprioritized — TRIAGE, do not blind-implement** (see §5).
- **Immediate next:** finish MODALITIES — `graph_theory` (verify), the `logic/` set (`owl`, `shacl_extensions`,
  `logic_modalities_shacl`, `specialized_libs_shacl`), `calculus/` (`ode_solver` math; GPU files need Timothy's
  answer). Then RDF modalities top-of-roadmap.

## 1. The four tracked docs (all on the branch unless noted)

| File | What |
|------|------|
| `.dev-docs/to-do/audit_production_excellence_tasks.md` | **The list.** ⚠ It is **git-ignored** (`.dev-docs/`), so check-offs live in the **MAIN checkout** (`C:\Projects\qualiaDB\.dev-docs\...`), NOT committed. Edit it there with `- [x] … _(✅ <commit> — …)_`. |
| `AUDIT_PRODUCTION_EXCELLENCE_PROGRESS_LOG.md` | Per-step results record (§0 triage explains the boilerplate-mismatch problem). |
| `AUDIT_BOUNDARY_DEFERRALS.md` | Everything intentionally NOT fully done + WHY + the 3 open questions for Timothy. |
| `RDF_REASONING_ROADMAP.md` | The second workstream (RDF-star, N3Logic built-ins, Datalog/RDFS-RL, ShEx, CogAI, GeoSPARQL, OWL-Time, SWRL, RIF, OWL EL/QL, SHACL-AF). |

## 2. Standing rules (CRITICAL — these are Timothy's, non-negotiable)

1. **Completeness bar (CLAUDE.md §11):** a reviewer must call it complete. **No `// TODO` / "honest follow-up" /
   `◑ partial` as a dodge — that is a task FAILURE.** The ONLY allowed deferral is a datum/decision only Timothy
   can supply, surfaced as ONE crisp ask. Genuine boundaries (research-scale, invariant conflict) → `[~]` + a row
   in `AUDIT_BOUNDARY_DEFERRALS.md`, never a fake tick.
2. **Split big files (CLAUDE.md §10):** building a file past ~450 lines → convert `foo.rs` → `foo/mod.rs` +
   submodules **as you go** (done for `abductive/`, `argumentation/`, `control_feedback/`). **Pre-existing
   monoliths** (`deontic.rs` ~1380, `graph_theory.rs` 930, `spatio_temporal.rs` ~620, `dialectical.rs`) →
   **deferred to a final "library-ization" pass**, flagged inline, NOT split mid-feature.
3. **Zero-heap or note:** every modality impl is zero-heap (slices in, scalars / caller `out` buffers / `u64`
   bitsets out) OR carries an inline `⚠ heap — off-hot-path` note. Only known heap: the Dung `argumentation/`
   library (hot-path primitive `grounded_contains` is zero-heap). See progress-log §8.
4. **QPU/quantum DEPRIORITIZED (WAP §0.11, memory `project-qpu-deprioritized`):** do NOT build new QPU capability
   (`solvers/qpu/**`, `quantum_optimizers/**`, any "QPU"/"quantum" audit bullets). This covers a chunk of SOLVERS.
5. **Commits:** as `mediaprophet`, **NO `Co-Authored-By` trailer** (Timothy is the author; tool ≠ co-author).
6. **`n3_parser.rs` is another worktree's allocation (`qualia-n3-parser`) — do NOT touch it.**

## 3. Build / test (CRITICAL gotchas — you WILL hit these)

- **Test cmd:** `RUST_MIN_STACK=134217728 cargo test -p qualia-core-db --lib -- modalities::<mod>`.
  The `RUST_MIN_STACK` is **REQUIRED** — without it rustc stack-overflows compiling the `trust-dns-proto`
  dev-dependency on Windows (`STATUS_ACCESS_VIOLATION`). `cargo check --lib` doesn't need it.
- **Multiple test filters go AFTER `--`:** `... --lib -- modalities::a modalities::b` (a single quoted
  multi-word filter matches 0 tests).
- **NEVER run GPU tests (WAP §0.10):** append `--skip <gpu_test_name>` (e.g. `--skip test_execute_diffusion_pass`).
  The A2000 is reserved for the LLM lane. → this is why `calculus/` GPU files are blocked (open question #2).
- **Git CWD RESETS** between turns / after task-notifications. **Always use absolute `git -C`:**
  `git -C C:/Projects/qualiaDB/.worktrees/qualia-prod-excellence <cmd>`. (A bare `git add` will hit the MAIN repo.)
- **Per-module loop:** read source → implement (zero-heap) → `cargo test … <mod>` (run in background) → on green
  `git -C <wt> commit` → check off the item in the MAIN-repo audit doc with the commit id.

## 4. Hard-won lesson: VERIFY BEFORE FLAGGING

Recalled memories can be **stale**. RCC-8 was flagged "unwireable" per memory — but `evaluate_rcc8_points`
already did exact float RCC-8 over a bounded vertex *slice*. **Always read the actual source before claiming
something is missing or a boundary.** Many "remaining" items are already done (check off after verifying) or are
boilerplate mis-pasted onto the wrong file (see §5).

## 5. The boilerplate-mismatch trap (READ before touching DOMAINS/SOLVERS/SPECIALIZED_LIBS)

The audit is auto-generated; large parts of DOMAINS/SOLVERS/SPECIALIZED_LIBS/OBFUSCATION repeat **identical**
"scope" bullets pasted onto unrelated files — e.g. the **economics** block ("predictive matrix of geopolitical
macro-events / Information-Banking tax clearing / fractional-resource optimization curves") on `geometric.rs`,
`thermodynamics.rs`, `convergence.rs`, QPU files; and the **bio** block ("Smith-Waterman / human-DNA privacy /
k-mer") on `linear_algebra/mod.rs`, `optimization/`, `obfuscation/hybrid_state_manager.rs`,
`logic_modalities_shacl.rs`, `specialized_libs_shacl.rs`. **Do NOT blind-implement these.** For each: implement
the file's REAL needed capability, and mark a genuinely mis-pasted bullet as misassigned (with the reason) —
that is honest, not a dodge. Note: `bioinformatics.rs` already HAS Smith-Waterman + Needleman-Wunsch;
`organic_chemistry.rs` (1807 LOC) and `chemistry_modeling.rs` (3173 LOC) are already doctorate-level → verify.

## 6. What's DONE (MODALITIES areas — committed, tested, checked off — do NOT redo)

epistemic-boundaries · jural · fuzzy · paraconsistent · modal · **capacity** (mechanism done; ⚑ domain
**vocabulary** is the open guardianship question) · linear (+ `webizen::ZkConsumeFact` opcode) · defeasible ·
**abductive/** (lib) · **argumentation/** (lib) · asp (3/4, CDNL `[~]`) · deontic · responsibility · delegation ·
contract · interaction_governance · legal_compose · deontic_compose · meta_deontic · consensus · carrier ·
identity_fabric (real Shamir SSS) · value_flow · capability_gap · diffusion · manifold_logic (3/4, GPU `[~]`) ·
**control_feedback/** (lib) · temporal_ltl · stit · probabilistic · epistemic · ctl (full CTL) · causal (full
do-calculus) · dl (2/4 structural; SROIQ-tableau `[~]×2`) · dialectical · spatio_temporal (RCC-8 verified done).
Plus the whole **EPISTEMIC BOUNDARIES** section.

## 7. What REMAINS

**MODALITIES (35 items, 10 files) — finish these first (mostly real):**
- `graph_theory.rs` (930 LOC, "COMPREHENSIVE") → likely **verify-and-check** + small gaps (PageRank/Louvain/
  subgraph-iso/bounded-memory). Pre-existing monolith → don't split (library-ization pass).
- `logic/owl.rs` (OWL 2 RL materialization, property-chain unrolling, disjointness quarantine) — real, doable.
- `logic/shacl_extensions.rs` (VC integration into SHACL targets, identity-as-enumerated-state, severity
  degradation, decentralized target routing) — real.
- `logic/logic_modalities_shacl.rs` + `logic/specialized_libs_shacl.rs` — **bio boilerplate** → verify
  (Smith-Waterman exists) + mark mis-pasted bullets.
- `calculus/ode_solver.rs` (symplectic integrators, BDF stiff solvers, dense output, sensitivity) — **pure math,
  doable, zero-heap.** Good next target.
- `calculus/cuda_bridge.rs`, `calculus/tensor_provenance.rs` (zk-SNARKs over tensors), `calculus/host.rs`
  (Smith-Waterman SIMD) — **GPU/zk → blocked on open question #2** (can I use the A2000, or CPU-side-only?).
- `n3_parser.rs` — **DEFERRED** (other worktree). Do not touch.

**Other sections (mostly TRIAGE per §5):** DOMAINS 24, SOLVERS 21 (much is QPU-deprioritized), SPECIALIZED_LIBS
12, OBFUSCATION 3. GEOMETRIC_ALGEBRA = 0 remaining.

**Then:** the RDF reasoning modalities in `RDF_REASONING_ROADMAP.md` (recommended order: RDF-star → N3Logic
built-ins → Datalog/RDFS-RL → ShEx → CogAI → GeoSPARQL/OWL-Time → SWRL/RIF/EL/QL/SHACL-AF).

## 8. ⚑ Open questions for Timothy (in `AUDIT_BOUNDARY_DEFERRALS.md`)

1. **Guardianship vocabulary** (`capacity.rs`) — (A) his canonical 17-domain list, or (B) approve a standard
   legal taxonomy as a renamable placeholder? Mechanism is complete; only the named domains are reserved to him.
2. **GPU tests** — may I verify the `calculus/` GPU items on the A2000, or do them CPU-side-only + mark the GPU
   path unverified? (Blocks `cuda_bridge`/`tensor_provenance`/`host`.)
3. **asp CDNL** — prioritize the clasp-grade solver engine as its own project, or leave it capability-complete `[~]`?

## 9. The 4 genuine boundaries so far (all in the boundary doc, NOT fake-ticked)
asp/CDNL performance · manifold GPU 10D renderer · dl SROIQ model-tableau (×2). All are research-scale and/or
zero-heap-conflicting, each with a recorded reason. (Soft notes on checked items: consensus full-BFT-protocol,
probabilistic junction-tree, ctl CTL*.)
