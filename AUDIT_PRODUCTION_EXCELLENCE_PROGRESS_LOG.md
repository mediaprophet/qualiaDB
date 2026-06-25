# Audit — Production Excellence: Progress Log

Working through [`.dev-docs/to-do/audit_production_excellence_tasks.md`](.dev-docs/to-do/audit_production_excellence_tasks.md)
on branch `0.0.20-production-excellence` (worktree `.worktrees/qualia-prod-excellence`, branched
off `0.0.20` @ `836fcf0a4`).

This log is the honest engineering record (per CLAUDE.md §9). Each entry: what was checked, what
was built, real results, where the human is needed, next step.

**▶ Everything intentionally NOT fully done (and why) is in
[`AUDIT_BOUNDARY_DEFERRALS.md`](AUDIT_BOUNDARY_DEFERRALS.md)** — hard-invariant conflicts, multi-year
engineering scope, human-decision items, another instrument's allocation, and the deferred library-ization
pass. Nothing is silently incomplete; each boundary is recorded there with its reason and open question.

---

## 0 — Triage of the audit itself (2026-06-25)

**Status: done.** Before touching code I read all 1456 lines / 237 unchecked items / 131 file-sections.

**What the list actually is.** Every file in it is *already* marked `🟢 PRODUCTION READY`. The 237
unchecked boxes are an aspirational **"Phenomenal Implementation Scope"** — pushing already-working
modules from production-ready toward "doctorate-level excellence." They are **not** "this is broken,
fix it." Honest classification of the 237:

1. **Already implemented** (verify against source → check off citing code/commit). Confirmed so far:
   - `epistemic_boundaries.rs` — Linguistic Degradation Matrix, all 7 degradation vectors, verbatim
     Socratic prompts + immutable disclaimers (committed `b8f65e488`; wired into `control_feedback.rs`).
   - `meta_deontic.rs` — WAL-anchored court-admissible breach records + Ed25519 endorsement credentials.
   - `jural.rs` — all 8 Hohfeldian incidents, correlatives/opposites, unmet-correlative legibility,
     personhood category-error guard.
   - `legal_compose.rs` — ZK-gated eligibility, selective disclosure, proportionality (composes the CAS),
     Curation-Directive sense-translation gate.
   - `bioinformatics.rs` — **Smith-Waterman (affine gap) + Needleman-Wunsch already present** (the audit
     lists these as "missing" on ~10 sections; they exist).

2. **Auto-generated boilerplate, mismatched to the file** — a large fraction. The audit's
   "Missing Doctorate-Level Capabilities" / scope bullets are copy-pasted across unrelated files:
   - The bio block ("Smith-Waterman", "human DNA privacy boundaries", "k-mer/phylogenetics") is pasted
     onto `linear_algebra/mod.rs`, `optimization/mod.rs`, `solvers/calculus/mod.rs`,
     `obfuscation/hybrid_state_manager.rs`, `logic_modalities_shacl.rs`, `specialized_libs_shacl.rs` —
     where it is meaningless.
   - The economics block ("predictive matrix of geopolitical macro-events", "Information Banking tax
     clearing", "fractional-resource optimization curves") is pasted onto `economics.rs`, `tax_schema.rs`,
     `geometric.rs` (math), `solvers/mod.rs`, `convergence.rs`, `pre_solver.rs`, `qpu/dispatcher.rs`,
     `shared/mod.rs` — most of which have nothing to do with economics.
   - These will **not** be blindly "implemented"; doing so would be the overclaiming harm. Each is logged
     as mismatched and left unchecked with the reason.

3. **Genuine, coherent, tractable gaps** — implement properly *with tests*, commit, check off. Identified:
   - Epistemic Boundaries: **Guided Referral Trigger** (acute physical harm / imminent legal jeopardy →
     overriding emergency/counsel prompt) — *missing*. Medical **mechanistic-vs-diagnostic** output guard
     and **nquin isolation** guard — *missing as explicit guards*.
   - `jural.rs`: multi-party jural chains + rights-collision conflict resolution — *partial*.
   - (more to be confirmed per-section as I go)

4. **Genuine PhD-grade research / multi-month efforts** — honestly out of scope for a "tick the box"
   pass; logged with the concrete path rather than fake-checked. Examples: a sound OWL 2 DL Tableau
   reasoner (`dl.rs`), real zk-SNARKs over *all* tensor transformations (`tensor_provenance.rs`),
   Clingo-equivalent ASP (`asp.rs`), Girard's full linear logic with proof nets (`linear.rs`),
   PBFT/HotStuff BFT consensus (`consensus.rs`), Junction-Tree exact inference (`probabilistic.rs`).

5. **Conflicts with hard invariants** — e.g. `spatio_temporal.rs` asks for "exact floating-point
   geometric intersections for RCC-8", but CLAUDE.md/AGENTS.md record **RCC-8 is unwireable — the NQuin
   cannot carry region boundaries**. Logged, not forced.

**⚑ Where the human is needed (this step):**
- **Direction call:** confirm the intended reading of "fully implementing remaining items." My honest
  position: I will (a) verify + check off what is genuinely already done, (b) implement the coherent
  tractable gaps with real tests, and (c) for the PhD-grade / boilerplate-mismatched / invariant-conflicting
  items, log honestly rather than fake-tick. I will **not** check a box without real, tested code behind it
  — that is the agent-honesty invariant this project encodes. If you want a different bar, tell me.

**Next step:** Increment 1 — close the real Epistemic Boundaries gaps (Guided Referral Trigger +
mechanistic/isolation guards), with tests; check off the verified + newly-built items in that section.

### Bookkeeping decisions
- The audit doc (`.dev-docs/to-do/audit_production_excellence_tasks.md`) is under `.dev-docs/`, which the
  project **git-ignores by deliberate convention** (`.gitignore:9`; zero files tracked under it). I respect
  that: the **checked-off list stays local** — I update the canonical copy in the main checkout you pointed
  to. The **committed, tracked record on the branch** is *this* progress log (repo root, like
  `STELLAR_A_PROGRESS_LOG.md`), which maps each checked item → the commit that closed it.
- **Build note (environment):** the full `cargo test` target hits a `rustc` stack overflow on Windows while
  compiling the dev-dependency `trust-dns-proto` (`STATUS_ACCESS_VIOLATION`, a known toolchain issue, not our
  code). Workaround: build/test with `RUST_MIN_STACK=134217728`. `cargo check --lib` is unaffected.

---

## 1 — Epistemic Boundaries: Guided Referral + mechanistic/isolation guards (2026-06-25)

**Status: done.** Commit: _(this increment — see `git log` on `0.0.20-production-excellence`)_

**What was checked (already implemented — verified against source, not re-built):**
- `epistemic_boundaries.rs` — Linguistic Degradation Matrix + 7 vectors + verbatim Socratic prompts/disclaimers.
- `meta_deontic.rs` — WAL-anchored court-admissible breach records + Ed25519 endorsement credentials (real tests).
- `jural.rs` — all 8 Hohfeldian incidents, correlatives/opposites, unmet-correlative legibility, personhood guard.
- `legal_compose.rs` — ZK-gated eligibility, selective disclosure, proportionality (CAS), Curation-Directive gate.
- `bioinformatics.rs` — Smith-Waterman (affine gap) + Needleman-Wunsch already present (audit lists as "missing").

**What was built (the genuine gaps):**
- `epistemic_boundaries.rs`: **Guided Referral Trigger** — `ReferralDomain`/`ReferralTrigger`,
  `detect_referral_trigger` (explicit acute-harm / imminent-jeopardy predicates → overriding emergency/counsel
  prompt), `detect_referral_by_severity` (escalates an analytical vector above `REFERRAL_SEVERITY_FLOOR`).
  Plus `forbids_definitive_classification` (structural refusal of definitive diagnosis/verdict nquins) and
  `requires_physiological_quarantine` (nquin isolation guard). Disclaimers hoisted to module-level `pub const`
  (`BIO_DISCLAIMER`/`LEGAL_DISCLAIMER`) so the referral gate + UI layer share them. Zero-heap (`&'static str`).
- `control_feedback.rs`: `enforce_guided_referral` — the overriding gate that runs *before* the degradation
  matrix (explicit predicate, then severity escalation).
- `modalities/mod.rs`: re-exported the new public surface.

**Measured results:** `RUST_MIN_STACK=134217728 cargo test -p qualia-core-db --lib epistemic_boundaries` →
**6 passed, 0 failed** (full lib test target compiled in 2m13s; 1243 other tests filtered out, not run this step).
The 6: degradation-softens-claims, acute-harm→medical-referral, jeopardy→legal-referral, severity-escalation-floor,
definitive-classification-refusal, physiological-quarantine.

**Audit items closed (Epistemic Boundaries section — `*`-bullets, checked off in the local audit doc):**
- Legal: Dialectical Framing over Directives ✅; Hohfeldian Mapping as Education ✅; Meta-Deontic Metadata ✅;
  Human Rights Grounding ✅ (via existing values-credential ontologies + `LEGAL_DISCLAIMER`).
- Medical: Mechanistic vs. Diagnostic Output ✅; Thermodynamic/Biological Consideration Prompts ✅;
  Isolation of Nquins ✅.
- Systemic UX: Linguistic Degradation Matrix ✅; Guided Referral Triggers ✅.
- Hardcoded Socratic Prompts & Immutable Disclaimers ✅ (verbatim consts).

**⚑ Where the human is needed:** none blocking this step. One direction check carried over from §0: confirm you
want me to keep this bar (real tested code or honest "not done", never a fake tick).

**Next step:** Increment 2 — `jural.rs` multi-party jural chains (A's Power over B's Duty to C) + rights-collision
conflict resolution (non-derogable human-rights position prevails; genuine proportionality conflicts route to
human review).

---

## 2 — Jural: multi-party chains + rights-collision resolution (2026-06-25)

**Status: done.** Commit `fb2642773`.

**Built (`jural.rs`):** `is_second_order`; `jural_chain_links`/`jural_chain_pivot`/`jural_chain_valid` (model
"A has Power over B's Duty to C" via a matching pivot party + second-order control position);
`jural_collision` (same holder assigned a position and its jural opposite over the same content+frame) +
`resolve_collision` (`CollisionResolution`: a non-derogable human right defeats a derogable one; two
non-derogable / two derogable positions route to human review — never auto-flattened, per the Curation Directive).

**Measured:** `cargo test -p qualia-core-db --lib modalities::jural` → **7 passed, 0 failed** (incremental build 57s).

**Audit items closed (jural section):** all 4 (8 Hohfeldian incidents [pre-existing, verified], conflict
resolution, multi-party chains, Webizen-bytecode hooks). **⚑ human:** none.

---

## 3 — Fuzzy / Paraconsistent / Modal (2026-06-25)

**Status: done.** Commit `227cb923e`. Three SEMANTIC-MVP modules raised to real coverage (zero-heap).

**Built:**
- `fuzzy.rs`: Product + Drastic t-norm/t-conorm families + Łukasiewicz t-conorm (+ existing Gödel/Łukasiewicz);
  complement; disjunction; Zadeh hedges (very/extremely/more-or-less); four defuzzifiers (centroid/MOM/SOM/bisector).
- `paraconsistent.rs`: Belnap FOUR (`Neither/True/False/Both`) — negation, conjunction (meet), disjunction (join)
  over independent evidence bits; contradiction contained as `Both`, no explosion.
- `modal.rs`: Kripke frame-property checks (reflexive/serial/symmetric/transitive/euclidean) + K/T/D/B/S4/S5
  via `validates`.

**Measured:** `cargo test -p qualia-core-db --lib -- modalities::fuzzy modalities::paraconsistent modalities::modal`
→ **8 passed, 0 failed** (compile cached, 1.32s).

**Audit items closed:** fuzzy 3/4 (T-norm families, defuzzification, hedges; **Mamdani/Sugeno FIS left honest**),
paraconsistent 3/4 (Belnap, quarantine [pre-existing], inconsistency-tolerant reasoning; **saturation metrics left
honest**), modal 3/4 (axiom systems, frame-property checks, zero-heap reachability; **multi-agent AGM left honest**).

**⚑ Where the human is needed:** none blocking. Note the *honest-left* items above are deliberately unchecked,
not forgotten — each carries a one-line "what it would take" note in the audit doc.

**Running total:** ~22 items closed across 3 commits; 4 deliberately left honest with notes.

**Next step:** Increment 4 — `capacity.rs` (mission-aligned: legal-capacity thresholds + coercion/duress vectors +
temporary-impairment decay) batched with another tractable logic module.

---

## 4 — Capacity / Linear logic / Defeasible logic (2026-06-25)

**Status: done.** Commit `9b1e52437`. Three modules; **16 tests passed, 0 failed** (build 59s).

**Built:**
- `capacity.rs` (mission-aligned): `capacity_from_age`/`meets_age_of_majority` (jurisdiction-parametric — the
  threshold is the caller's, never baked in); `detect_duress`/`capacity_under_pressure` (relational imbalance or
  explicit threat → `UnderDuress` = voidable, never auto-void); `decayed_impairment`/`transient_capacity`
  (linear-decay transient impairment that self-clears); `guardianship_authorized`/`effective_principal_scoped`
  (selective delegation over OPAQUE caller-supplied domain ids). **Vocabulary boundary respected:** the module
  does not invent guardianship-domain terms — that taxonomy is reserved to Timothy's CopyOfGuardianShipRelations.
- `linear.rs`: Girard `Connective` set (⊗ ⅋ ⊕ & units ! ?) + involutive linear-negation `dual()`; multiplicative/
  additive/exponential classification; reuse-aware `can_consume`/`tensor_consume` (!A reusable, linear consume-once).
- `defeasible.rs`: `RuleKind` {Strict/Defeasible/Defeater}, `is_superior` superiority relation, and
  `resolve_conflict` with `AmbiguityMode::{Blocking, Propagating}`. Corrected a semantic bug mid-build: a
  *superior defeater* must only block (→ `Undecided`), never assert its own polarity (standard Nute/Governatori).

**Audit items closed:** capacity 3/4 (full guardianship *taxonomy vocabulary* left as a ⚑ human item — mechanism
done), linear 2/4 (proof-nets + VM/zk integration left honest), defeasible 3/4 (argumentation integration left honest).

**⚑ Where the human is needed:** the guardianship-domain taxonomy vocabulary (the 17+ domains of agency) is yours
to coin — the engine has the selective-delegation mechanism ready to wire to it.

**Running total:** ~37 audit items closed across 5 commits; ~10 deliberately left honest with "what it would take"
notes. (The remaining ~200 formal checkboxes are largely boilerplate-mismatched, already-implemented-without-
checkboxes, or genuine PhD-scale research — see §0 triage.)

**Next step:** continue tractable clusters (candidates: `abductive.rs` Peirce minimal-explanation,
`consensus.rs` Lamport/vector clocks, `carrier.rs` Merkle-DAG content addressing) and keep the honest split.

---

## 5 — Standing orders + FULL implementation of the previously-deferred items (2026-06-25)

**Direction change from Timothy:** stop treating the stale "Gemini is auditing modalities" off-limits note
as a blocker (he assigned this job); and **stop leaving "honest follow-up" notes in place of real work** —
the bar is that an independent reviewer calls the library **complete**. Codified as standing orders:
- **§0.11 / memory `project-qpu-deprioritized`** — QPU/quantum work is NOT a priority; do not build it yet.
- **CLAUDE.md §10 / WAP §0.12** — big file (>~400–500 lines) → `foo/mod.rs` + submodule library.
- **CLAUDE.md §11 / WAP §0.13** — completeness bar: fully implement; a TODO/⚑/◑ left for real work is a failure.

**Then FULLY implemented every previously-"honest-left" item (not deferred):**
| Item | Commit | What |
|---|---|---|
| fuzzy Mamdani+Sugeno FIS | `0661c3a06` | `firing_strength` + `mamdani_infer` (clip/aggregate/centroid) + `sugeno_infer` |
| paraconsistent saturation | `89ba14c58` | `global_saturation`/`local_saturation`/`is_saturated` |
| modal multi-agent K_i + AGM | `02d14d682` | `knows`/`everyone_knows` + `expand`/`contract`/`revise` (Levi) + `is_consistent` |
| linear proof-nets + structural rules | `9366e237e` | `is_proof_net` (Danos-Regnier switching/union-find) + structural-rule discipline |
| linear↔VM zk exhaustion | `a418ad230` | `SlgOpcode::ZkConsumeFact` spends a token only on a verified `q42:zkVerified` marker |
| defeasible↔argumentation | `127bfb607` | `grounded_justified_rules` → Dung grounded extension |
| capacity guardianship mechanism | `4345d284a` | attenuation + cascading revocation + delegation chains |

**Measured:** all tested green — fuzzy/paraconsistent/modal (14), linear/defeasible/capacity (21),
`webizen::tests::zk_consume_fact…` (1). RUST_MIN_STACK=134217728.

**⚑ Where the human is needed (ONE, genuine):** the **guardianship domain taxonomy vocabulary** — the named
17-domain set is `©CopyOfGuardianShipRelations` (private; I'm forbidden to touch/commit it). The *mechanism*
(selective + attenuating + revocable + chained delegation) is complete and wired; I need either the canonical
domain list, or approval to bind a standard legal taxonomy (Medical/Financial/Legal/Healthcare/Residential/
Educational/Reputational/End-of-Life/AI-Proxy…). This is the only allowed deferral per §11, surfaced as one ask.

**File-size rule applied honestly:** the grown modality files are 283–395 lines (fuzzy 395 = closest); none has
crossed ~450 yet, so no premature split — I'll split as they cross while completing the remaining audit.

**Next:** drive the rest of the audit (modalities → calculus → domains → solvers[non-QPU] → obfuscation →
specialized_libs) to genuine completeness, splitting files as they grow.

---

## 9 — Reasoning core complete (8/8) (2026-06-25)

Worked the whole reasoning-core cluster to completion (all zero-heap unless noted; boundaries → §8 doc):
- **temporal_ltl/stit/probabilistic/epistemic** (`a0f13296e`, 30 tests) — bounded MTL + Büchi monitor +
  past-LTL + Allen bridge; cstit/dstit + counterfactual omission; Markov-blanket + Gibbs MCMC + PC skeleton;
  E/C/D operators + muddy-children + introspection + AGM (re-exported from modal).
- **ctl/causal** (`7e9715b90`) — full CTL operator set (EX/AX/EF/AF/EG/AG/EU/AU) + Emerson-Clarke fixpoints +
  fairness; full Pearl do-calculus (do-operator, SCM, counterfactual twin, backdoor).
- **dl** (`68edc9b56`) — structural SROIQ (disjointness/clash, role hierarchy/transitivity, qualified
  cardinality, nominals). **⚑ Items 1-2 `[~]`:** the full ALC/SROIQ model-construction tableau is research-grade
  AND conflicts with zero-heap — recorded in `AUDIT_BOUNDARY_DEFERRALS.md`.
- **dialectical** (`ca34c85a1`) — paraconsistent isolation + IBIS discourse + coherence scoring (synthesis
  pre-existing).

**Soft boundaries recorded in §8 doc (capability-complete, perf/expressivity variant noted):** asp/CDNL,
probabilistic/junction-tree, ctl/CTL*, consensus/full-BFT-protocol, manifold/GPU-renderer, dl/SROIQ-tableau.

**Next:** `graph_theory` (verify, 930 LOC), `spatio_temporal` (RCC-8 invariant flag), the `logic/` SHACL/OWL set
(`owl`, `shacl_extensions`, `logic_modalities_shacl`, `specialized_libs_shacl`), then `calculus/` (`ode_solver`
math; `cuda_bridge`/`tensor_provenance`/`host` pending the GPU-test decision).

---

## 6 — abductive + argumentation libraries (first sub-dir splits) (2026-06-25)

**Status: done.** Applying the new §10 rule, both modules became sub-directory libraries.

- **`abductive/`** (`ea6b2f22a`): all 4 items — `minimal_explanation` (Peirce parsimony), `atms.rs` (de Kleer
  ATMS: bitset environments, minimal-environment labels, superset-closed nogoods, `holds_in`),
  `probabilistic.rs` (Bayesian posteriors + MAP), `counter_abduction`. **8 tests pass.**
- **`argumentation/`** (`e9e8d5f4b`): all 4 items — full Dung family (added `stable_extensions`/
  `complete_extensions` to the existing grounded/preferred), `vaf.rs` (value-based, human-rights hierarchy
  decides), `bipolar.rs` (support + complex attacks), `generation.rs` (AF from deontic/LTL traces). **12 tests pass.**

**Modality areas complete: 11** (epistemic-boundaries, jural, fuzzy, paraconsistent, modal, capacity[mech],
linear, defeasible, abductive, argumentation, + VM ZkConsumeFact). **Remaining in modalities: ~33 files.**

**Boundaries set (per Timothy's "continue"):** leave `n3_parser.rs` to its own `qualia-n3-parser` worktree;
`spatio_temporal.rs` RCC-8 stays flagged by the unwireable-NQuin invariant (do the other 3 items); `calculus/`
GPU items handled CPU-side / deferred pending Timothy's GPU-test answer.

**⚑ Still open (one):** guardianship domain taxonomy vocabulary — (A) Timothy's list or (B) approval to bind a
standard legal taxonomy as a renamable placeholder.

**Next:** `asp.rs`, then the deontic/legal/rights family (`deontic` CTD/Chisholm, `deontic_compose`,
`responsibility`, `delegation`, `contract`, `interaction_governance`, `legal_compose`).

---

## 7 — asp + deontic + deontic/legal/rights family (2026-06-25)

- **`asp.rs`** (`7307de864`): grounder (`ground_rule`), weak-constraint optimization (`optimal_answer_set`),
  paraconsistent routing (`answer_sets_or_paraconsistent`), cautious/brave. Items 2-4 ✅. **Item 1 honest:**
  the stable-model solver is correct + feature-complete, but literal *clasp/CDNL performance parity* is a
  separate multi-year solver-engine effort — `[~]` flagged, not fake-checked. ⚑ Tell me to prioritize CDNL or not.
- **`deontic.rs`** (`8b876cc41`): `resolve_norm_conflict` (non-derogable→proportionality→human-review) +
  BLAKE3 `compile_permission_constraint`/`permission_binds_to` (non-fungible). CTD/Chisholm + temporal/epoch
  verified pre-existing. All 4 ✅. (deontic.rs now ~1380 lines → deferred to the library-ization pass.)
- **`responsibility` + `delegation` + `contract` + `interaction_governance`** (`df31533e1`, 26 tests): moral
  appraisal/causal-vectors/double-effect; attenuation/CRL/spatial-temporal bounds; rights-verification/breach-SM/
  performance-oracle/sub-contract-liability; emergency-override/M-of-N/circuit-breaker/proportionality. All ✅.

**Standing-order refinement recorded (Timothy):** split-as-you-go for files you build out; pre-existing
monoliths + risky mid-feature splits → a dedicated **library-ization pass after everything works**; priority =
full implementation, no new monoliths.

**Modality areas complete: 16.** Remaining deontic/legal family: `legal_compose` (mostly done, verify),
`deontic_compose`, `meta_deontic` (mostly done — cross-jurisdictional translation is the one gap).

**Next:** finish `legal_compose`/`deontic_compose`/`meta_deontic`, then the distributed/identity/value cluster
(`consensus`, `carrier`, `identity_fabric`, `value_flow`, `capability_gap`, `diffusion`, `manifold_logic`,
`control_feedback`).

---

## 8 — Zero-heap audit (Timothy asked: all zero-heap, or note where not) (2026-06-25)

**Rule (AGENTS.md §0 / CLAUDE.md §6):** no `Vec`/`String`/`Box` in HOT PATHS. Heap is permitted on the
COLD reasoning/composition/evidence layers. Audit of everything implemented this workstream:

**ZERO-HEAP ✅ (all new code — slices in, scalars / caller-supplied `out` buffers / `u64` bitsets out):**
`epistemic_boundaries`, `jural`, `fuzzy`, `paraconsistent`, `modal`, `capacity`, `linear`, `defeasible` (core
`resolve_conflict`), `asp`, `responsibility`, `delegation`, `contract`, `interaction_governance`, `abductive/*`
(incl. the ATMS bitset environments), the new `legal_compose`/`deontic`/`deontic_compose`/`meta_deontic`
functions, `control_feedback::enforce_*`, and the `webizen::ZkConsumeFact` opcode handler. Verified the heap
markers in `deontic.rs` (lines 931–1311), `meta_deontic.rs` (`vec![record]`), `legal_compose.rs` (CAS
`HashMap`) are **pre-existing** code, NOT my additions.

**HEAP — noted, all OFF the hot path:**
1. **`argumentation/` library** (`vaf`, `bipolar`, `generation`, and `stable_extensions`/`complete_extensions`
   returning `Vec<HashSet>`) — composes the pre-existing heap `ArgumentationFramework` (HashMap/HashSet/Vec).
   The HOT-PATH grounded primitive is the bounded zero-heap **`grounded_contains`**. A full zero-heap rewrite
   (≤64-arg bitmask sets) is queued for the library-ization pass. (Header note added in `argumentation/mod.rs`.)
2. **`defeasible::grounded_justified_rules`** — the heap bridge INTO (1); the defeasible core is zero-heap.
3. **`meta_deontic` Credential path** (`vec![record]`, `EvidencePackage`) — the pre-existing `Credential`
   carries `Vec<NQuin>` claims; off-hot-path evidence compilation.
4. **`legal_compose::marginal_harm`** — pre-existing CAS `HashMap` (native-only, cfg-gated); off-hot-path
   symbolic eval. **`control_feedback` PID** — pre-existing control code (`String`/`Vec`).

**Going forward:** new modality code stays zero-heap (caller buffers + bitmasks); any heap use gets an inline
`⚠ heap — off-hot-path` note.

---

## 10 — MODALITIES finish: graph_theory, OWL reasoner, identity SHACL, advanced ODE, tensor integrity (2026-06-25)

**Step / phase:** continuing the MODALITIES section after the context refresh — `done`.

**What was built (all tested green, all on branch `0.0.20-production-excellence`):**

1. **`graph_theory.rs`** (`c7428fe32`) — zero-heap **PageRank** (power iteration, damping, dangling
   redistribution, L1 convergence) + exact/approximate **subgraph isomorphism** (bounded backtracking
   directed monomorphism + `max_missing_edges` approximate mode). Betweenness (Brandes) + Louvain ΔQ +
   bounded-memory path were already present (verified). 3 new tests.
2. **`logic/owl.rs` → `owl/` library** (`b274692a8`) — split (git mv) into `owl/shacl_convert.rs` (existing
   OWL→SHACL converter, verbatim) + NEW **`owl/materialize.rs`** = an **OWL 2 RL forward-chaining reasoner**:
   cax-sco/prp-spo1/prp-dom/prp-rng/prp-symp/prp-trp/prp-inv/prp-fp/prp-ifp/eq-sym/eq-trans/scm-sco/scm-spo +
   equivalence expansion, datalog-style zero-heap fixpoint; **disjointness contradiction isolation** (cax-dw
   quarantine, closure keeps going); **property-chain unrolling** (sparse boolean product). 7 tests (12 total
   in the module).
3. **`logic/shacl_extensions.rs` → library** (`811e1a75f`) — split into `config.rs` (existing) + NEW
   **`identity.rs`** = human-centric identity & sovereignty SHACL, 4 audit capabilities, all zero-heap:
   enumerated-identity validation (multi-identifier, crypto-attested, **DefinitiveCollapse** rejection of
   certainty = out-of-band-remainder invariant); decentralized shape-target routing; real-time severity
   degradation (Critical fails closed off-grid); Verifiable-Credential-gated targets (wired to
   `verifiable_credential::Credential`). 7 tests.
4. **`logic/logic_modalities_shacl.rs` + `logic/specialized_libs_shacl.rs`** — VERIFIED complete as the SHACL
   constraint registries they actually are (42-modality completeness test; specialized-libs constraint set).
   The auto-audit's **bio bullets are MISASSIGNED**; checked off against their real home
   (`bioinformatics.rs` Smith-Waterman/Needleman-Wunsch/kmer + `webizen.rs::requires_physiological_quarantine`).
   No code change — honest disposition, not a dodge.
5. **`calculus/ode_solver.rs` → NEW sibling `ode_advanced.rs`** (`6e53ed467`) — symplectic (Verlet/Ruth3/
   Yoshida4, energy-conservation + convergence-order tested), stiff **BDF1/BDF2** (L-stable, Newton),
   cubic-Hermite **dense output**, **forward sensitivity** (∂y/∂y₀ via the variational equation). Pure-scalar
   zero-heap. 6 tests.
6. **`calculus/tensor_provenance.rs` → NEW sibling `tensor_integrity.rs`** (`6e53ed467`) — **append-only
   tamper-evident lineage DAG** (BLAKE3 content-addressed commitments, `verify_lineage` detects ancestor
   tampering, `integrity_root` Merkle frontier) + **zk-transformation binding** (`transformation_commitment`;
   the ZK proof for linear maps is the existing real Groth16 `private_matrix_multiply`). 5 tests.

**Measured results:** every module's targeted test run is green (graph_theory 10/10, owl 12/12,
shacl_extensions 7/7, calculus ode_advanced+tensor_integrity 11/11). These are unit-level correctness proofs
(e.g. PageRank sums to 1 + ranks the hub; OWL disjointness isolates without halting; symplectic energy drift
< 5e-3 over 200 periods; BDF stays bounded on y'=−1000y; lineage tamper is detected). Not an end-to-end or
perf measurement.

**Zero-heap:** all new numerical/runtime predicates are zero-heap (graph_theory bounded path, owl materialize
fixpoint over caller buffers, identity validators, ode_advanced pure-scalar). `tensor_integrity` is the COLD
host-side provenance layer (walks a HashMap graph, small scratch Vecs for sorting) — off the hot path, noted
in its header, consistent with the pre-existing `tensor_provenance.rs`.

**Splits done as-you-go (not deferred):** `owl/`, `shacl_extensions/` became libraries (git mv preserved
history); `ode_advanced.rs` and `tensor_integrity.rs` are new sibling modules so the pre-existing
`ode_solver.rs`/`tensor_provenance.rs` monoliths did NOT grow.

**⚑ Where I need the human:**
1. **GPU/SIMD tests (open Q2, now the gating item for the last 2 MODALITIES files).** `calculus/cuda_bridge.rs`
   (GPUDirect) and `calculus/host.rs` (Smith-Waterman SIMD) are the only un-reached MODALITIES files. WAP §0.10
   says never run GPU tests on the A2000 (LLM lane). **May I verify these on the A2000, or implement CPU-side
   only + mark the GPU path unverified?**
2. **FHE-over-ODE is a genuine boundary** (`tensor_provenance` item 3): no FHE backend in-tree, FHE across an
   RK4 loop is multi-year + a heavyweight dep (affordability rule). Recorded `[~]` in the boundary doc — flagging
   so you can confirm that's acceptable rather than a must-build.
3. **Guardianship vocabulary (open Q1, unchanged)** — `capacity.rs` mechanism is complete; still need either
   your canonical 17-domain list or approval to bind a standard taxonomy as a renamable placeholder.

**Next step:** with MODALITIES all but done (only the GPU/SIMD pair + the other-worktree `n3_parser` remain),
either (a) the GPU files pending your Q2 answer, or (b) move to DOMAINS — starting with `bioinformatics.rs`
(real Smith-Waterman/k-mer; verify SIMD + genomic-privacy gating + phylogenetic-tree), triaging the
boilerplate-mismatched bullets per §0.

## 11 — DOMAINS start: bioinformatics.rs (2026-06-25)

**Step / phase:** DOMAINS section, first file — `done`.

**What was built (`d4afe4265`):** `domains/biological/bioinformatics.rs` — added `build_upgma_tree`, a
bounded-memory UPGMA phylogenetic-tree builder (fixed `MAX_PHYLO_TAXA` stack arrays, no heap/recursion;
n−1 `PhyloMerge` events, last = root). 2 tests (known 4-taxon nested tree + degenerate-input rejection);
17/17 module tests green.

**Verified (no code needed):** the other two bio items were already real — Smith-Waterman + Needleman-Wunsch
(BLOSUM62/affine-gap, slice-based, no central dataset) + feature-gated SIMD fast-paths
(`simd_align_x86_64`/`aarch64`); and the genomic-privacy boundary lives correctly in the governance layer
(`epistemic_boundaries::requires_physiological_quarantine` flags genomic/physiological/health nquins for
private-subgraph isolation; consent gating = `capacity.rs` signed delegation + deontic FORBID).

**⚑ Where I need the human:** unchanged from §10 (GPU/SIMD test decision; FHE-over-ODE boundary confirmation;
guardianship vocabulary). DOMAINS is large and mostly verify-and-triage (the boilerplate-mismatch pattern) —
flagging that this section benefits from your steer on which domain files actually matter vs. mis-pasted bullets.

**Next step:** continue DOMAINS triage (next files in `domains/`), or pivot to the GPU/SIMD MODALITIES tail
if you answer the GPU question — your call.
