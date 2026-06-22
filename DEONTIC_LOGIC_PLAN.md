# Computational Deontic / Legal-Logic Stack — Implementation Plan

**Status: living document. Created 2026-06-22.**
Owner agent builds; **Timothy directs and has final say.** This file exists so the build does
not drift or get abandoned half-finished across turns. Work top-to-bottom by phase; update the
checkboxes and the status table *only after tests run green*.

---

## 0. Honesty contract (read first)

This plan was prompted by a fair complaint: earlier framing implied the deontic stack was
"all done." It was not. To prevent that recurring:

1. **No item is marked ✅ Done until its test runs green** and the test command + result are
   recorded in the commit message. "Compiles" ≠ "works".
2. **Standalone ≠ composed.** A modality module existing (`fuzzy.rs`, `asp.rs`, …) does NOT
   mean it is wired into the deontic verdict pipeline. Those are tracked separately.
3. **Gemini's spec contains invented symbol names.** Where the spec cites an engine symbol
   that does not exist, this plan says so and maps to the real one (see §Name-map).
4. **Architectural changes (new opcodes, NQuin bit-layout) are proposed here and confirmed by
   Timothy before landing** — they are not migrated unilaterally.

Legend: ✅ real+tested · ◑ partial / standalone-not-composed · ✗ absent · 🔬 exists, claims
unverified this cycle (must run its tests in Phase 0 before relying on it).

---

## 1. Current state — honest audit (verified against source 2026-06-22)

| # | Capability (Gemini spec) | Status | Where / note |
|---|--------------------------|--------|--------------|
| 1 | SDL triad O/P/F | ✅ | `deontic.rs:145` OP_OBLIGATE/PERMIT/FORBID; `F≡O¬` |
| 1 | Optionality `U`, Gratuitousness `G` | ✗ | derivable from O/P; no operator |
| 1 | KD axioms (K, D, N, E) | ✗ / by-design | engine is an **evaluator, not a theorem-prover**. Axiom-D consistency is **detected** (paraconsistent flagging), not assumed — a deliberate divergence to document, not "fix" |
| 2 | Dyadic / conditional `O(q\|p)` | ◑ | only the CTD special case exists; no general conditional operator |
| 2 | Contrary-to-Duty `O(q\|¬p)` | ✅ | `deontic.rs:521` `evaluate_contrary_to_duty` (breach→reparation, fact-scan) |
| 3 | Defeasible — rebutting | ✅ | `q42:unless` / `DEFEATER_BIT` (bit 63) |
| 3 | Defeasible — undercutting | ✗ | only rebutting; no link-invalidation |
| 4 | Temporal deontic `O(Xp/Fp/Gp/p U q)` | ◑ | expiry→`Expired` ✅; `temporal_ltl.rs` exists standalone; **not composed** with O |
| 5 | Action/agency STIT `[α stit p]` | ◑ | norms carry bearer `borneBy`/`heldBy` ≈ binding; no stit operator, no joint action |
| 6 | Hohfeld — Immunity↔Disability | ◑ | Immunity ✅ (`illocution.rs`, non-derogable); Disability correlative ✗ |
| 6 | Hohfeld — Claim↔Duty, Privilege↔No-Right, Power↔Liability | ✗ | not modeled as correlative relations |
| 7 | Lifecycle: Pending / Violated / Discharged | ✗ | `DeonticStatus` = Active/Defeated/Expired/Malformed only |
| 7 | Epistemic-deontic (mens rea) | ✗ | `epistemic.rs` + `deontic.rs` exist separately; **no composition** |
| 8 | Spatial-deontic (locative O, jurisdictional subsumption) | ◑ | `spatio_temporal.rs` + `jurisdiction.n3` exist; deontic composition ✗. NB **RCC-8 region geometry is unwireable in a 48-byte NQuin** (prior finding) → use the `jur:within` hierarchy, not topological calculus |
| 9 | Argumentation (Dung grounded extension) | 🔬→wire | `argumentation.rs` exists; verify + wire to deontic conflict |
| 10 | Linear / resource-bounded (discharge, unmet duty) | ◑ | `linear.rs` exists standalone; discharge→lifecycle + unmet-correlative-duty ✗ |
| 11 | Meta-deontic (provenance, crypto endorsement, WAL record) | ◑ | provenance ✅ in corpus; ed25519 endorsement ✅ (`verifiable_credential.rs`); `wal.rs` ✅; **not composed into the verdict** |
| 12 | Description Logic (subsumption, disjointness, category-error) | 🔬 / ◑ | `dl.rs` exists; `PersonhoodDisjointnessShape` ✅ (values.n3); forward-chaining category-error trigger — verify |
| 13 | Probabilistic / Fuzzy (partial fulfillment, trust) | 🔬→wire | `fuzzy.rs`, `probabilistic.rs` exist; not wired to fulfillment/trust |
| 14 | ASP / Abductive (stable models, diagnosis) | 🔬→wire | `asp.rs`, `abductive.rs` exist; not wired to remedy-choice / breach-diagnosis |
| 15 | Interaction Governance (policy modes) | ◑ | Phase-8 DenyRollback + WAL exist; `DeonticVerdict → {PreventiveBlock, PermissiveAudit, Prioritize, Interactive}` mapping ✗ |

**Summary:** the SDL *core* is real; ~8 modality engines exist standalone but **uncomposed**
with deontic; ~6 genuinely-missing pieces (U/G operators, undercutting, lifecycle states,
Hohfeld correlativity, STIT operator, mens-rea composition).

---

## 2. Name-map (Gemini spec → real engine, or INVENTED)

| Gemini cited | Reality |
|--------------|---------|
| `spatio_temporal::evaluate_rcc8` | `spatio_temporal.rs` exists; **no RCC-8 geometry** (NQuin can't carry region boundaries). Use `jur:within`. |
| `NativeEpistemicEval` | INVENTED. Real: `epistemic::evaluate_epistemic_frame` |
| `modalities::dl::multiple_inheritance_dag` | verify exact fn in `dl.rs` (likely different name) |
| `values:RemedyStrippingFlag`, `values:dutyToVerify`, `MandatoryBaseline`, `VoidableStipulation`, `CoercedConsentFlag`, `EffectivityInterval`, `SanctionKind`, `claimedIdentityUnverifiable` | NOT YET in the ontology — to be **coined with Timothy** (sensitive vocab; do not invent unilaterally) |
| `values_evaluate` lifecycle, `BreachRecord` | lifecycle states to be ADDED (Phase 1); `BreachRecord` = WAL entry, to be defined (Phase 5) |
| `argumentation::grounded_extension` | verify exact fn in `argumentation.rs` |

---

## 3. Architectural budget (confirm with Timothy before Phase 1 lands)

- **Opcodes:** deontic owns `0x10–0x12`. New deontic opcodes take `0x13–0x1F` (same block):
  proposed `0x13 OP_OPTIONAL`, `0x14 OP_GRATUITOUS`, `0x15 OP_CONDITIONAL` (dyadic head),
  `0x16 OP_STIT` (agency tag, OR a metadata bit — see Phase 3). Hohfeld jural relations are
  **predicates/relations, not opcodes** (Phase 2).
- **Lifecycle** extends `DeonticStatus` (an enum, `#[repr(u8)]`) — additive, no layout change:
  `Pending=0x04, Violated=0x05, Discharged=0x06`.
- **Zero-heap:** all evaluators stay slice-in / slice-out (no Vec/Box in hot paths), matching
  `evaluate_deontic_contract`.
- **Undercutting** needs a second defeater kind — propose a metadata bit (rebut vs undercut)
  rather than a new opcode.

---

## 4. Phased roadmap

### Phase 0 — Honesty baseline (no new features) ✅ DONE 2026-06-22
- [x] Ran the existing tests per standalone modality. **Command:**
  `cargo test -p qualia-core-db --lib modalities::` → **159 passed, 0 failed.** Per-module:
  dl 2 · fuzzy 1 · probabilistic 1 · asp 2 · abductive 1 · argumentation 6 · linear 1 ·
  spatio_temporal 4 · temporal_ltl 6 · paraconsistent 1 · epistemic 3 · logic::deontic 15.
  **Finding:** all standalone modalities are REAL (green tests), but several are thinly
  covered (fuzzy/probabilistic/abductive/linear/paraconsistent = 1 test each) and **none are
  composed with the deontic verdict** — that composition is Phase 4. 🔬 markers in §1 → ✅
  *standalone* (not ✅ *composed*).

### Phase 1 — Deontic core extensions  [`deontic.rs`] ✅ DONE 2026-06-22
**Test:** `cargo test -p qualia-core-db --lib modalities::logic::deontic::` → **21 passed**
(was 15); full lib suite **1090 passed, 0 failed**.
- [x] Lifecycle states: `Pending(0x04) / Violated(0x05) / Discharged(0x06)` (additive enum) +
  `norm_lifecycle_status()` (effectivity → expiry → defeater → facts {fulfilled→Discharged,
  breached/performed→Violated}) + tests.
- [x] `Optionality (U, 0x13)` / `Gratuitousness (G, 0x14)`: opcodes + derived `is_optional`
  (`¬O ∧ ¬F`) / `is_gratuitous` (`¬O`) + tests.
- [x] Undercutting defeater (`OP_UNDERCUT 0x17`) vs rebutting — classified via new
  `DefeatKind` on the verdict (opcode-based, no layout change; metadata bit not needed) + tests.
- [x] First-class dyadic `O(q|p)` — `evaluate_conditional_obligation()`; CTD refactored to a
  special case; `OP_CONDITIONAL 0x15` reserved + tests.

### Phase 2 — Hohfeldian jural square  [`modalities/jural.rs` + `core-ontologies/jural.n3`] ✅ DONE 2026-06-22
**Test:** `cargo test … modalities::jural::` → **5 passed**; jural.n3 ingests (76 triples);
full lib suite **1095 passed, 0 failed**.
- [x] 8 positions (opcodes 0x30–0x37): Claim↔Duty, Privilege↔No-Right, Power↔Liability,
  Immunity↔Disability; `correlative()` (involutive) + `jural_opposite()`.
- [x] NQuin encoding (`compile_jural_quin`: holder→subject, counterparty→object,
  content|position→predicate, frame→context) + `correlative_quin()` + `jural_correlativity_holds()`.
- [x] `find_unmet_correlatives()` — "make the absence legible": a Claim with no Duty-bearer
  surfaces the missing duty (feeds Phase 4 linear unmet-correlative-duty).
- [x] `personhood_category_error()` — a non-NaturalPerson asserting a human-only Claim/
  Privilege/Immunity, composing `dl::check_subsumption_quin` disjointness.
- [x] `core-ontologies/jural.n3` curated vocabulary mirroring the Rust.

### Phase 3 — STIT agency  [`modalities/stit.rs`] ✅ DONE 2026-06-22
**Test:** `cargo test … modalities::stit::` → **4 passed**; full lib suite **1099 passed, 0 failed**.
- [x] Agent-bound `O[α stit φ]` / `F[α stit φ]` over the deontic norm Quin (subject = α, the
  causal force); `brought_about` fact convention `(α, q42:broughtAbout, φ)`.
- [x] `is_duty_bearer` (duty-bearer vs bystander) + `agentive_status` (post-hoc: brought
  about → Discharged; obligation not met → omission → Violated; forbidden act done → Violated).
- [x] Joint action `O[{α,β} stit φ]`: `joint_discharged` (any member suffices) +
  `joint_liable_members` (shared liability — all members liable if undischarged).

### Phase 4 — Compositions with deontic (the bulk of §8–10, §13–14)  [`modalities/deontic_compose.rs`]
**Cluster A (zero-heap) ✅ DONE 2026-06-22** — `cargo test … modalities::deontic_compose::`
→ **5 passed**; full lib suite **1104 passed, 0 failed**.
- [x] deontic × temporal: `obligation_globally` (`O(Gφ)`) + `obligation_until` (`O(φ U ψ)`)
  via `temporal_ltl::evaluate_ltl_trace`.
- [x] deontic × epistemic: mens rea — `MensRea {Knowing, Ignorant, InexcusableIgnorance}` +
  `classify_mens_rea` + `agent_knows` (ignorantia juris non excusat when duty-to-know).
- [x] deontic × spatial: `obligation_applies_in` — jurisdictional subsumption via `jur:within`
  (DL transitive closure; not RCC-8 geometry, per §1).
- [x] deontic × linear: `discharge_obligation` — fulfilment → `Discharged` AND consumes the
  duty (`linear::consume_quin`). Unmet-correlative-duty already shipped in Phase 2
  (`jural::find_unmet_correlatives`).
**Cluster B (heap-using reasoning engines) — remaining:**
- [ ] deontic × argumentation: conflicting norms → grounded extension → final verdict.
- [ ] deontic × probabilistic/fuzzy: partial fulfillment `μ∈[0,1]`; trust threshold.
- [ ] deontic × ASP/abductive: multi-remedy stable models; breach → minimal-cause diagnosis.

### Phase 5 — Meta-deontic  [`wal.rs` + provenance]
- [ ] Provenance anchoring + ed25519 endorsement folded into the verdict (Curation Directive).
- [ ] `BreachRecord` → WAL (court-admissible, Merkle-DAG linked) + tests.

### Phase 6 — Interaction Governance  [Webizen VM]
- [ ] `DeonticVerdict → PolicyMode {PreventiveBlock(DenyRollback) | PermissiveAudit(WAL) |
  Prioritize(hict QoS) | Interactive(sense:HumanCorrection)}` + tests.

### Phase 7 — Surface (no logic in the UI; mirror the engine)
- [ ] Modalities Observatory demo cards for each new capability (faithful to the Rust).
- [ ] values-credentials.html panels (jural square; lifecycle; mens rea).
- [ ] MCP tools exposing the new evaluators.

### Sensitive-vocab gate (Timothy)
- [ ] Coin the contested ontology terms with Timothy before use: `dutyToVerify`,
  `RemedyStrippingFlag`, `MandatoryBaseline`, `CoercedConsentFlag`, `VoidableStipulation`,
  `SanctionKind`, `claimedIdentityUnverifiable`. (Per project norm: defer sensitive coinage.)

---

## Track M (parallel, first-class) — MCP agent cooperation  [task #17/#16/#18]

Flagged by Timothy: this has been orphaned and that's not acceptable — it is load-bearing for
the whole thesis (agents verifying each other's conduct; trust behaviourally derived; no
platform-provider). It is **not** gated behind the deontic work; run it in parallel.

- [ ] Every MCP call bound to a **verified typed calling-agent identity** + standpoint
  (who is asking, in what role/frame), not anonymous.
- [ ] Caller identity verified (signed VC / key), not asserted (ties to `verifiable_credential.rs`
  `verify_grounded` — ungrounded-AI-issuer already rejected).
- [ ] Standpoint recorded on the interaction record (the interactionism locus, #16).
- [ ] Deontic gate: an MCP caller's request is evaluated against the rights ontology before
  execution (this is where Track M meets Phase 6).

---

## Anti-drift protocol
- One phase (or sub-item) = one commit; tests green; message records the test command + result.
- Tick the checkbox here in the same commit. Never tick without a green test.
- If a phase reveals a needed architectural change, STOP and confirm with Timothy (§3 budget).
- Keep this file as the single source of truth for "what's actually done."
