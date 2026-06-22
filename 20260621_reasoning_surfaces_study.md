# Reasoning-surfaces census — `qualia-core-db` (2026-06-21)

A comprehensive study of every reasoning / computational modality in the engine, to answer
"is the values implementation too narrow?" and map what's available. Surveyed:
`src/modalities/` (+ `calculus/`, `logic/`, `logic/shacl/`), `src/solvers/` (+ subdirs),
`src/specialized_libs/` (+ subdirs), and reasoning-relevant `src/*.rs`.

Maturity read off size: **●** substantial (≥200L / many pub fns), **◐** moderate, **○** thin/MVP
(~30–140L, 1–2 fns). "values" = wired+tested to the values layer this session; "→" = genuinely
values-relevant, not yet wired.

---

## A. Logic modalities (`modalities/`) — ~20

| modality | file | size | primary API | values status |
|---|---|---|---|---|
| Deontic (Obligate/Permit/Forbid) | `logic/deontic.rs` | ● 966L | `evaluate_deontic_contract`, `compile_n3_rule_to_norm`, `evaluate_contrary_to_duty` | ✅ wired (norms + remedy) |
| Defeasible (`unless`) | `defeasible.rs` | ◐ 107L | `DEFEATER_BIT`, `OP_DEFEASIBLE_OVERRIDE` | ✅ wired |
| Forward-chaining guards | `webizen.rs::fire_guard_rules` | ● | premise-join → conclusion | ✅ wired (G1, from file) |
| Interval (Allen) | `interval_reasoning.rs` | ● 500L f22 | `TemporalInterval` (contains/overlaps/…) | ✅ wired (effectivity) |
| Spatio-temporal (Allen + **RCC-8**) | `spatio_temporal.rs` | ● 497L | `evaluate_rcc8`, `SpatialRegion` | ✅ wired (jurisdiction) |
| Paraconsistent | `paraconsistent.rs` | ◐ 108L | `route_paraconsistent` (isolate, no ex-falso) | ✅ wired (conflict) |
| Argumentation (Dung) | `argumentation.rs` | ● 514L f17 | `grounded_extension`, `preferred_extensions`, `argument_status` | ✅ wired (rights-conflict) |
| **Epistemic** (knowledge/locks) | `epistemic.rs` | ◐ 222L | `evaluate_epistemic_frame`, `check_node_locks`, `EpistemicStatus` | → identity-as-known |
| **Modal** (◇/□) | `modal.rs` | ○ 67L | `possible`, `necessary` | → identity is MODAL (Timothy's point) |
| **Fuzzy** | `fuzzy.rs` | ○ 64L | `t_norm_godel/lukasiewicz`, `degree` | → vague legal standards ("reasonable", "adequate", "proportionate") |
| **Description Logic** | `dl.rs` | ○ 67L | `check_subsumption_quin` | → the Agent lattice (NaturalPerson ⊑ Agent) subsumption |
| **CTL** (branching time) | `ctl.rs` | ○ 131L | `exists_finally`, `always_globally` | → obligations over possible futures |
| **Abductive** | `abductive.rs` | ○ 64L | `abductive_explanation` | → "why was this flagged" |
| **ASP** (stable models) | `asp.rs` | ○ 71L | `enumerate_stable_models` | → consistent-scenario enumeration |
| **Probabilistic** | `probabilistic.rs` | ◐ 216L | `evaluate_threshold`, Bayesian-ish | → trust/reputation (trustfactory) |
| Dialectical | `dialectical.rs` | ● 493L | dialectical reasoning | → complements argumentation |
| Temporal LTL | `temporal_ltl.rs` | ◐ 287L | `evaluate_ltl_trace`, `holds_within`, `LtlFormula` | → trace-temporal (lock/lease) |
| Diffusion (spreading activation) | `diffusion.rs` | ○ 137L | `trigger_diffusion` | (assoc. retrieval) |
| Linear logic (consumed resources) | `linear.rs` | ○ 33L | CONSUMED_BIT | → consent/one-shot tokens |
| Graph theory | `graph_theory.rs` | ● 929L f9 | centrality/paths on NQuin graphs | → relation-graph analysis |
| Control/feedback | `control_feedback.rs` | ● 478L | control theory | (other domains) |
| Calculus (ODE + GPU) | `calculus/` | ● ~3700L | `ode_solver`, tensor-provenance | (physics/quant) |
| QUBO | `logic/qubo.rs` | ◐ 299L | semantic→QUBO | (quantum offload) |

The VM + symbolic plumbing also live here: `logic/core.rs` (Webizen bytecode VM), `n3_parser`,
`n3_compiler`, `n3logic` (router), `owl` (OWL→SHACL), `rules`, and the SHACL-extension files.

## B. Solvers (`solvers/`) — numeric / optimization / SAT, subject-matter-selected

`calculus/` (differential), `linear_algebra/` (matrix, f25), `optimization/` (root-finding, f9),
`quantum_optimizers/`, `qpu/` (8-provider quantum dispatch + pre-solver), `symbolic_logic/`
(SAT / symbolic, 985L — note: this is SAT/logic, **not** algebra), `shared/convergence`.

## C. Specialized domain libraries (`specialized_libs/`) — the DOMAIN modalities

| domain | file | size | values relevance |
|---|---|---|---|
| **Algebra / CAS** | `symbolic_algebra.rs` | ◐ 669L | ✅ wired (proportionality + Expr↔NQuin provenance) |
| **Economic / financial** | `financial_modeling.rs` | ● 4123L | ✅ wired (CAS core); rich lib available (portfolios/risk) |
| **Statistics** | `statistical_computing.rs` | ● 2846L | → disparate-impact, sampling, privacy-preserving stats |
| **Machine learning** | `machine_learning.rs` | ● 3527L | → fraud/risk classification |
| **Medical** | `medical_computing.rs` | ● 4470L | → health-rights evidence (+ `clinical_engine.rs`, `comorbidity_eval.rs`) |
| **Physics / Chemistry / Engineering** | resp. files | ● ~3200L each | → forensic / environmental evidence |
| **Cryptographic** | `cryptographic_library.rs` | ● 4762L | identity/signing primitives (trustfactory) |
| Linear algebra | `linear_algebra.rs` (+ submodules) | ● 1302L | spectral/consistency scoring |
| Quantum (bridge, biology) | `qpu_bridge/`, `quantum_biology/` | ● | (specialist) |
| Zero-heap utils | `shared/zero_heap.rs` | ● 421L f34 | (infra) |

## D. Reasoning / governance surfaces in `src/*.rs` (beyond modalities)

- **`epistemic.rs` (441L) — Dynamic Epistemic Logic (DEL)**: separates *objective vs subjective*
  reality. This is the identifier≠identity / standpoint substrate — a SECOND epistemic surface
  (richer than `modalities/epistemic.rs`). → trustfactory / identity.
- **`deontic_logic.rs` (524L) — ODRL policy evaluation** for credential-gated subgraphs (the
  agreements/policy layer; distinct from `modalities/logic/deontic.rs`). → §4.4 contract route.
- **`deontic_circuit.rs` — Deontic Access Circuit (arkworks ZK)** + **`zk_proofs.rs`**: ZK proofs
  of deontic compliance. → privacy-preserving eligibility (§19).
- **`portal_standpoint.rs` — "Human-Centric observer standpoint; the chosen context and right to
  perceive"**: the *perspectival* dimension — the modal "from whose view" of identity (§13).
- `provenance.rs` (labor DID + contestability), `webizen_validator.rs`, `semantic_culler.rs`
  (agency-driven filtering), `comorbidity_eval.rs` (defeasible medical), `quantum_dft.rs`,
  `qubo_compiler.rs`, `rdf_star.rs`, `mini_parser.rs`, `ontology_loader.rs`.

---

## Verdict — is the values implementation too narrow?

**Yes — even at ten wired, the engine is far broader.** Census: ~20 logic modalities, ~7 solver
families, ~12 domain libraries, plus DEL / ODRL / ZK / standpoint surfaces in `src/`. The values
layer wires **10**; the engine offers materially more, and several unwired ones are **genuinely
values-load-bearing**, in priority order:

1. **Epistemic + Modal + Standpoint** — identity-as-known, ◇/□, observer perspective. The
   identifier≠identity / trustfactory spine (§13). Two real surfaces (`modalities/epistemic.rs`,
   `src/epistemic.rs` DEL) + `modal.rs` + `portal_standpoint.rs`.
2. **Description Logic (`dl.rs`)** — subsumption over the Agent lattice (NaturalPerson ⊑ Agent;
   PublicAuthority/CorporatePerson). Makes the personhood hierarchy itself machine-reasoned.
3. **Fuzzy (`fuzzy.rs`)** — vague legal standards are pervasive ("reasonable", "adequate",
   "proportionate", "cruel"). Degrees of compliance, not just binary.
4. **Probabilistic** — behaviourally-derived trust (trustfactory).
5. **Abductive / ASP / Dialectical** — explanation ("why flagged"), scenario enumeration,
   and dialectical resolution (complementing argumentation).
6. **Domain libs** (statistics, ML, medical, physics…) — subject-matter-selected evidence.

**Maturity caveat (honest):** several logic modalities are thin/MVP (`abductive`, `asp`, `dl`,
`modal`, `fuzzy`, `diffusion` ≈ 30–140L, 1–2 fns) — they exist and the routing is real, but
wiring them to values will likely surface gaps to fill, exactly as the smoke tests have done.
The substantial ones (argumentation, interval, spatio-temporal, dialectical, epistemic,
probabilistic, graph-theory, calculus, + all domain libs) are mature.

**Next breadth priority: epistemic + modal + DL** — they complete the *identity/personhood*
spine (the §13 identifier≠identity principle made computable), which is the highest-value
unwired cluster.
