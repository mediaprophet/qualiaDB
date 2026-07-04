# Act III — Modalities

> *Thirty-plus formal logics, one wire format.*

---

## Thesis

> **The engine does not pick one logic. It carries thirty of them. They share
> the same wire format, the same arena, the same opcode space. They do not
> call each other; they iterate over the same slices.**

---

## Voice-over script

### Shot 1 — A grid of modality names appears, arranged like a periodic table. [SLOW]

> These are the reasoning systems compiled into the engine. [PAUSE]
> They are not stubs. They are not "coming soon." They are in the binary,
> and they have tests. [PAUSE]

### Shot 2 — Each modality lights up, one at a time. As it lights, a one-line description appears. [ITEM]

> Deontic logic — obligation, permission, prohibition, with a defeater bit. [PAUSE] [ITEM]
> Epistemic logic — what an agent knows, what an agent believes, what is
> common knowledge. [PAUSE] [ITEM]
> Paraconsistent logic — contradictions are isolated into a quarantine
> context; the rest of the system keeps running. [PAUSE] [ITEM]
> Linear temporal logic — globally, finally, next, until, release, over a
> real trace. [PAUSE] [ITEM]
> Computation tree logic — exists, always, until, reachable, fair. [PAUSE] [ITEM]
> Modal logic — possible, necessary, with five frame classes. [PAUSE] [ITEM]
> Description logic — subsumption, disjointness, cardinality, nominals. [PAUSE] [ITEM]
> Answer set programming — stable models, weak constraints, cautious
> consequences. [PAUSE] [ITEM]
> Defeasible logic — rules that can be overridden, with explicit
> superiority. [PAUSE] [ITEM]
> Linear logic — resources that are consumed when used. [PAUSE] [ITEM]
> Fuzzy logic — Gödel, Łukasiewicz, product, drastic, with hedges. [PAUSE] [ITEM]
> Probabilistic logic — Bayesian networks, Gibbs sampling, Markov blankets. [PAUSE] [ITEM]
> Dialectical logic — thesis, antithesis, synthesis, with the Hegelian
> coherence check. [PAUSE] [ITEM]
> Causal logic — but-for cause, overdetermination, do-calculus, backdoor
> adjustment. [PAUSE] [ITEM]
> Contract logic — formation, performance, breach, liability tracing. [PAUSE] [ITEM]
> Capacity logic — age of majority, duress, guardianship authority. [PAUSE] [ITEM]
> Delegation logic — authority flow, attenuation, revocation, CRL. [PAUSE] [ITEM]
> Responsibility logic — adjudication, mens rea, double effect. [PAUSE] [ITEM]
> Jural logic — correlativity, personhood, collision resolution. [PAUSE] [ITEM]
> Identity fabric — Shamir secret sharing, zk capability grants, web of
> trust. [PAUSE] [ITEM]
> Value flow — commons cost, royalty, EROI, usury ceiling. [PAUSE] [ITEM]
> Consensus — BFT quorum, Lamport happens-before, equivocation. [PAUSE] [ITEM]
> Illocution — speech acts, directive weight, exemptive norms. [PAUSE] [ITEM]
> STIT — see-to-it-that, joint action, counterfactual prevention. [PAUSE] [ITEM]
> Quantum lattice — propositions, orthocomplement, measurement. [PAUSE] [ITEM]
> Manifold — ten-dimensional coordinates, LTL over the trace, ASP stable
> models. [PAUSE] [ITEM]
> Manifold logic — Vietoris-Rips, persistent H₀, wave evaluation. [PAUSE] [ITEM]
> Interval reasoning — Allen's thirteen relations, constraint solving. [PAUSE] [ITEM]
> Spatio-temporal — RCC-8 regions, centroids, area, intersection. [PAUSE] [ITEM]
> Diffusion — belief propagation, energy injection, annealing. [PAUSE] [ITEM]
> Graph theory — community detection, betweenness, motif discovery. [PAUSE] [ITEM]
> Argumentation — IBIS positions, dialectical contradiction. [PAUSE] [ITEM]
> Abductive — explanation by best fit. [PAUSE] [ITEM]
> Control feedback — bang-bang, PID, with stability proofs. [PAUSE] [ITEM]
> Calculus — symbolic differentiation, integration, limits, ODEs. [PAUSE] [ITEM]
> Likeliness — Bayesian estimation, capability gap, learning paths. [END LIST] [PAUSE]

### Shot 3 — All thirty-six modalities pulse together for one beat. Then dim. [SLOW]

> Thirty-six. [PAUSE]
> One wire format. [PAUSE]
> One arena. [PAUSE]
> One opcode space. [PAUSE]

### Shot 4 — A single NQuin is shown. The predicate field's low byte is highlighted. [SLOW]

> Every modality gets its opcode in the low byte of the predicate. [PAUSE]
> Zero-point-one-zero is obligate. Zero-point-one-one is permit. Zero-point-one-two
> is forbid. Zero-point-two-zero is knows. Zero-point-two-one is believes.
> Zero-point-three-zero is isolate. Zero-point-four-zero is globally. [PAUSE]
> The opcode space above zero-point-one is reserved for new modalities. [PAUSE]
> The parser owns zero-point-zero through zero-point-four. It does not change. [PAUSE]

### Shot 5 — Two modalities fire on the same Quin slice. The verdicts are written to the same output buffer. [SLOW]

> They do not call each other. [PAUSE]
> They iterate over the same slices. [PAUSE]
> The caller decides which modality to invoke. [PAUSE]
> The caller decides how to combine the verdicts. [PAUSE]

### Shot 6 — Title card: **Thirty-six logics. One binary.** [SLOW]

> This is not a research project. [PAUSE]
> This is a working engine. [PAUSE]

---

## On-screen notes

- **Shot 1:** A periodic-table-style grid. Each cell is a modality. The cells are color-coded by family (deontic, epistemic, temporal, modal, probabilistic, geometric, etc.).
- **Shot 2:** Each modality lights up for ~4 seconds. A one-line description appears beneath. The camera does not move; the highlight does.
- **Shot 3:** All cells pulse together once, then dim. The viewer sees the scale.
- **Shot 4:** A single NQuin, predicate field highlighted. The opcodes are listed as text overlays.
- **Shot 5:** Two evaluations running on the same slice. The output buffer is a fixed `[Verdict; N]` stack array.
- **Shot 6:** Title card.

---

## Source code anchors

- `crates/qualia-core-db/src/modalities/mod.rs` — the modality registry.
- `crates/qualia-core-db/src/modalities/deontic_logic.rs` — `OP_OBLIGATE = 0x10`, `OP_PERMIT = 0x11`, `OP_FORBID = 0x12`, 10/10 tests.
- `crates/qualia-core-db/src/modalities/epistemic.rs` — `OP_KNOWS = 0x20`, `OP_BELIEVES = 0x21`, `OP_COMMON_KNOWLEDGE = 0x22`.
- `crates/qualia-core-db/src/modalities/paraconsistent.rs` — `OP_ISOLATE = 0x30`, `OP_CONTRADICTION_SCORE = 0x31`, `OP_PARACONSISTENT_MERGE = 0x32`.
- `crates/qualia-core-db/src/modalities/temporal_ltl.rs` — `OP_LTL_GLOBALLY = 0x40`, `OP_LTL_FINALLY = 0x41`, `OP_LTL_NEXT = 0x42`, `OP_LTL_UNTIL = 0x43`, `OP_LTL_RELEASE = 0x44`.
- `crates/qualia-core-db/src/modalities/ctl.rs`, `modal.rs`, `dl.rs`, `asp.rs`, `defeasible.rs`, `linear.rs`, `fuzzy.rs`, `probabilistic.rs`, `dialectical.rs`, `causal.rs`, `contract.rs`, `capacity.rs`, `delegation.rs`, `responsibility.rs`, `jural.rs`, `identity_fabric.rs`, `value_flow.rs`, `consensus.rs`, `illocution.rs`, `stit.rs`, `quantum_dft.rs`, `manifold.rs`, `manifold_logic.rs`, `interval_reasoning.rs`, `spatio_temporal.rs`, `diffusion.rs`, `graph_theory.rs`, `epistemic_boundaries.rs`, `interaction_governance.rs`, `legal_compose.rs`, `deontic_compose.rs`, `meta_deontic.rs`, `fuzzy_quantifiers.rs`, `fuzzy_rdf_schema.rs`, `fuzzy_type2.rs`, `carrier.rs`, `capability_gap.rs`.
- `crates/qualia-core-db/src/modalities_lite/` — the WASM-safe subset.
- `AGENTS.md §2`, `§3`, `§4` — the canonical task map and known-bug list.

---

## Duration

Approximately 180 seconds. This is the longest montage. It is also the most
defensible: every modality is a real file with real tests.
