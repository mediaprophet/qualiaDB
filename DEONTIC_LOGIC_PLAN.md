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
**Cluster B ✅ DONE 2026-06-22** (all zero-heap — the engines use bounded fixed arrays, not
heap, contrary to the earlier guess) — `modalities::deontic_compose::` → **9 passed** (A+B);
full lib suite **1108 passed, 0 failed**.
- [x] deontic × argumentation: `norm_survives_conflict` — Dung grounded extension picks the
  verdict winner (general duty reinstated when an override defeats its exception). Composes
  `argumentation::grounded_contains`.
- [x] deontic × probabilistic/fuzzy: `fulfilment_degree` / `obligation_fuzzily_met` (Gödel
  t-norm, progressive realization) + `trust_gate` (`probabilistic::evaluate_threshold`).
- [x] deontic × ASP/abductive: `remedy_scenarios` (under-determined remedy → stable models)
  + `diagnose_breach` (backward chain to root cause). Composes `asp` + `abductive`.

### Phase 5 — Meta-deontic  [`modalities/meta_deontic.rs`] ✅ DONE 2026-06-22
**Test:** `cargo test … modalities::meta_deontic::` → **3 passed**; full lib suite **1111 passed, 0 failed**.
- [x] Provenance anchoring: `build_breach_record` stamps the source instrument in `context`
  (`prov:wasDerivedFrom`); `breach_provenance` reads it back.
- [x] `BreachRecord` → WAL: `record_breach_to_wal` writes a `Violated` verdict to the
  Merkle-DAG–linked WAL (court-admissible); round-trips via `wal::recover` (tempfile-tested).
- [x] ed25519 endorsement (Curation Directive): `endorsement_credential` wraps the record as
  a `Credential` claim, signed by the identity layer (`verifiable_credential::issue`) and
  verified (`::verify`) — engine never holds keys; tamper-detection tested.

### Phase 6 — Interaction Governance  [`modalities/interaction_governance.rs`] ✅ DONE 2026-06-22
**Test:** `cargo test … modalities::interaction_governance::` → **5 passed**; full lib suite **1116 passed, 0 failed**.
- [x] `map_policy(status, Governance{non_derogable, humanitarian, ambiguous}) → PolicyMode
  {PreventiveBlock(DenyRollback) | PermissiveAudit(WAL) | Prioritize(hict QoS) |
  Interactive(HumanCorrection) | Allow}`. Pure decision layer (effects performed by caller).
- [x] Precedence: ambiguity → human; non-derogable violation → block; ordinary violation →
  audit; in-force humanitarian → prioritize; else allow. `permits_execution` go/no-go bit +
  `policy_action` labels. (Reusable by both the VM and Track M's MCP gate.)

### SHACL coverage (cross-cutting) ✅ DONE 2026-06-22
**Test:** `cargo test … logic_modalities_shacl` → **3 passed**; full lib suite **1118 passed, 0 failed**.
- [x] `logic_modalities_shacl.rs` `q42:<Name>ConfigurationShape` added for every SDL⁺ construct:
  DeonticLifecycle, DeonticExt (Optionality/Gratuitousness/Conditional/Undercut + DefeatKind),
  Jural, Stit, MensRea, InteractionGovernance, MetaDeontic (`LOGIC_MODALITY_SHAPES` 21→28).
- [x] **Epistemic shape enriched** (was `certainty`-only) → opcodes 0x20–0x22, the 9 named
  certainty bands, nesting depth, world/agent scoping. Completeness tests gate engine↔SHACL parity.

### Phase 7 — Surface (no logic in the UI; mirror the engine) ✅ DONE 2026-06-22
**Test:** `mcp_tool_impls::tests::` → 11 passed; full lib suite **1120 passed, 0 failed**;
Observatory cards verified live in-browser.
- [x] Modalities Observatory demo cards (`modality-engine.js`, 21→25): Hohfeldian Jural Square,
  STIT Agency, Mens Rea, Interaction Governance — faithful JS mirrors, verified in-browser.
- [x] MCP tools: `jural_correlate` (Hohfeld correlativity) + `deontic_govern` (verdict →
  PolicyMode), registered in `mcp_server.rs`, tested. (The category-error is already exposed
  via `values_check`; the deontic VM via `values_evaluate`.)
- [~] values-credentials.html panels — deferred (lower value; the Observatory is the live
  demo surface and the values page already covers the rights corpus). Reopen if wanted.

### Sensitive-vocab gate (Timothy)
- [ ] Coin the contested ontology terms with Timothy before use: `dutyToVerify`,
  `RemedyStrippingFlag`, `MandatoryBaseline`, `CoercedConsentFlag`, `VoidableStipulation`,
  `SanctionKind`, `claimedIdentityUnverifiable`. (Per project norm: defer sensitive coinage.)

---

## Track M (parallel, first-class) — MCP agent cooperation  [task #17/#16/#18]

Flagged by Timothy: this has been orphaned and that's not acceptable — it is load-bearing for
the whole thesis (agents verifying each other's conduct; trust behaviourally derived; no
platform-provider). It is **not** gated behind the deontic work; run it in parallel.

**Mechanism ✅ DONE 2026-06-22** — `mcp_cooperation.rs` + `mcp_cooperate` tool —
`cargo test … mcp_cooperation::` → 5 passed; mcp tools 12 passed; full lib suite **1126 passed, 0 failed**.
- [x] `CallerStandpoint {agent, role, verified}` — who is asking, in what typed role, and
  whether the identity was **verified** (signed VC) vs merely asserted.
- [x] `authorize` / `authorize_call` gate: verified-not-asserted → grounded (agency.n3 G1',
  composes `agent::is_ungrounded_agency`) → Phase-6 deontic policy. Verdicts: Authorized /
  DeniedUnverified / DeniedUngrounded / DeniedByPolicy.
- [x] `mcp_cooperate` MCP tool exposes the gate (registered in `mcp_server.rs`, tested).
- [ ] **Mandatory per-call enforcement in the dispatch** — fail-closed on unverified callers
  for EVERY tool call. This is a deliberate breaking MCP-contract change → **awaiting Timothy's
  sign-off** before wiring into `enforce_fiduciary_tool_dispatch` (the mechanism is ready).

---

## Phases 8–N — §16–§30 (the extended legal-logic stack, from `legal_logic.md`)

Same honesty contract. Built in clusters by value + buildability. **§18 (capacity / duress /
guardianship / posthumous) and §20 (wave-physics manifold) are gated** — §18 on Timothy's
sensitive-vocab decisions (DV / coercion / guardianship), §20 on the STELLAR manifold tasks.

### Phase 8 — Liability & accountability cluster ✅ DONE 2026-06-22
**Test:** `modalities::causal::` 3 + `modalities::responsibility::` 2; full lib suite **1131 passed, 0 failed**.
- [x] §16 Causal & counterfactual (`causal.rs`): `but_for_cause` (sine-qua-non), `is_voided_by`
  / `dependents_voided` (root-node dependency cascade — deepest-absence), `is_overdetermined`
  (joint liability when no single cause is but-for). Bounded BFS, zero-heap.
- [x] §25 Meta-statement (`responsibility.rs`): `ResponsibilityStatus {Alleged, Adjudicated,
  Dismissed}` + `adjudicate` + `is_enforceable_fact` (an allegation is NOT a fact until
  adjudicated — stops accusation-as-weapon).
- [x] §30 Systemic meta-guard (`responsibility.rs`): `rule_of_law_asymmetry`,
  `enforcer_overreach` (no appeal path), `accountability_vacuum` — the person protected from
  the system.

### Phase 9 — Capacity, delegation, contract ✅ DONE 2026-06-22
**Test:** capacity 4 + delegation 2 + contract 3; full lib suite **1140 passed, 0 failed**.
- [x] §18 `capacity.rs` (over EXISTING vocab): `CapacityStatus {Intact, Impaired, UnderDuress}`,
  `stipulation_binding` (Intact only), `stipulation_voidable` (duress → **voidable at victim's
  election, NOT auto-void** — ✓ CONFIRMED by Timothy 2026-06-22), `effective_principal` (guardian carries the dependent's
  weight), `posthumous_standing` (deceased + representative).
- [x] §21 `delegation.rs`: `has_delegated_authority` (transitive delegation chain) +
  `authority_after_revocation` / `revoked_descendants` (**revocation cascade** — revoke upstream,
  every downstream dependent is defeated; independent chains survive). Bounded BFS, zero-heap.
- [x] §22 `contract.rs`: `FormationStage {None, Offer, Binding}` + `is_binding_contract`
  (composes §18 — binding needs assent AND both parties' capacity intact) +
  `incorporates_by_reference`.

### Phase 10 — Economic, capability, identity ✅ DONE 2026-06-22
**Test:** value_flow 3 + capability_gap 2 + identity_fabric 2; full lib suite **1147 passed, 0 failed**.
- [x] §23 `value_flow.rs`: `commons_cost` (production cost + **capped** ROI), `royalty` (scaled
  by agent category), `pool_after`, `is_commons_discharged` (pool ≥ cost → freed globally),
  `outstanding`. Integer, deterministic.
- [x] §24 `capability_gap.rs`: `capability_gap` (Req \ Holds set-difference) + `requirements_met`,
  with experiential `skos:closeMatch` equivalence closing the gap (RPL).
- [x] §27 `identity_fabric.rs`: `identity_survives_loss` (k-of-n quorum recovery),
  `recompute_fabric` (re-compute identity from surviving anchors after key loss/theft),
  `identifier_is_not_identity` axiom.

### Phase 11 — Composition wires ✅ DONE 2026-06-22  [`modalities/legal_compose.rs`]
**Test:** `modalities::legal_compose::` 3 passed; full lib suite **1150 passed, 0 failed**.
- [x] §17 ZK-gated eligibility (`zk_eligibility` over `zk_proofs` Groth16 verification result) +
  `selective_disclosure` (reveal only chosen credential claims).
- [x] §26 Proportionality — `marginal_harm` / `proportionality_met` genuinely compose the CAS
  (`symbolic_algebra::parse`+`differentiate`+`eval`): `∂Harm/∂x < Advantage`.
- [x] §19 Sense-translation gate — `translation_status {CloseMatch, ExactMatch,
  RequiresHumanReview}` enforcing the Curation Directive (machine proposes closeMatch; only a
  human attests exactMatch; untranslatable → human review).

### Deferred / heavier
- [x] §28 Distributed state / consensus ✅ `modalities/consensus.rs` (3 tests; full suite
  **1153 passed**): `transaction_status` (multi-party commits only on full consensus),
  `is_globally_valid` (local ≠ global until synced), `survives_partition` /
  `can_form_joint_during_partition` (partition tolerance).
- [x] §20 wave-physics **logic** ✅ `manifold_logic.rs` (2 tests): `wave_eval` (Ψ at a coord),
  `integrate_abs`, `continuous_to_fact` (`∫Ψ > τ → Fact(p)` — the continuous→discrete bridge to
  epistemic.rs). **Substrate remaining:** the GPU-enumerated 10D-tensor *renderer* = STELLAR #11–13.
- [x] §29 multi-modal binding **logic** ✅ `carrier.rs` (3 tests): `media_tag` (real BLAKE3
  content-address), `verify_binding` (tamper-evident media↔graph binding), `extract_payload`.
  **Substrate remaining:** the binary container *codecs* (PDF/A-3, XMP, PNG) = task #9.

---

## Anti-drift protocol
- One phase (or sub-item) = one commit; tests green; message records the test command + result.
- Tick the checkbox here in the same commit. Never tick without a green test.
- If a phase reveals a needed architectural change, STOP and confirm with Timothy (§3 budget).
- Keep this file as the single source of truth for "what's actually done."
