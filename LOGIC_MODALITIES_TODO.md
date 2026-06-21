# Logic Modalities & ZK — To-Do Backlog

Living backlog, captured 2026-06-21. Grounded in a survey of
`crates/qualia-core-db/src/modalities/` + `modalities/logic/` and the Webizen VM
opcode handlers in `webizen.rs::execute_vm_frame`. Feeds the
`PendingImplementation` / MCP backlog model (no mocks — build for real or mark
pending).

The **gold standard** is the deontic lane: a modality is "done" when it has
(a) a real library impl, (b) a real VM opcode handler that calls it, and
(c) a live-lane round-trip test (parse → register → fire → `Native*Eval` →
asserted verdict), like `tests/deontic_smoke.rs`.

---

## 1. Modality status matrix

| Modality | Library | Tests | VM opcode | Handler | Gap |
|---|---|---|---|---|---|
| **deontic** | `logic/deontic.rs` | 13 | `NativeDeonticEval` | **real** | ✅ reference impl |
| **epistemic** | `epistemic.rs` | 2 | `NativeEpistemicEval(u8)` | **real** | live-lane smoke test |
| **linear** | `linear.rs` | 1 | `NativeLinearConsume` | **real** | live-lane smoke test |
| **asp** (answer-set) | `asp.rs` | 1 | `NativeAspStableModels` | **real** | live-lane smoke test |
| **paraconsistent** | `paraconsistent.rs` | 1 | `NativeParaconsistentIsolate` | **real** | live-lane smoke test |
| **dialectical** | `dialectical.rs` | 6 | `NativeDialecticalSynthesis` | **real** | review 2 stub markers |
| **defeasible** | `defeasible.rs` | 1 | `NativeUnless` | **real** | live-lane smoke test |
| **temporal LTL** | `temporal_ltl.rs` (`evaluate_ltl_trace`) | 5 | `NativeLtlGlobally/Finally/Next/Until/Release` | **real** ✅ (wired 0.0.19) | live-lane test in `tests/modalities_active.rs` ✅ |
| **Allen interval / RCC-8** | `spatio_temporal.rs`, `interval_reasoning.rs` | 3 + 6 | `NativeAllenInterval(u8)` | **real** ✅ (wired 0.0.19) | RCC-8 spatial opcode still unwired; review 5+2 stub markers |
| **probabilistic** | `probabilistic.rs` | 1 | `NativeProbabilisticThreshold(u32)` | **real** ✅ (wired 0.0.19) | belief-threshold gate; Bayesian-network inference not yet a lane |
| **description logic (DL)** | `dl.rs` | 1 | `NativeDlSubsumption` | **real** ✅ (wired 0.0.19) | transitive subClassOf; richer TBox (∩/∃/¬) later |
| **argumentation** | `argumentation.rs` | 5 | `NativeArgumentationGrounded` | **real** ✅ (wired 0.0.19) | Dung grounded semantics; preferred/stable extensions later |
| **diffusion** | `diffusion.rs` | 1 | none | — | decide: needs a VM lane? (`qualia:diffuse` block) |
| **graph theory** | `graph_theory.rs` | 7 | none | — | library-only; fine? |
| **control feedback** | `control_feedback.rs` | 4 | none | — | review 1 stub marker |

### Highest-value items (§1)
1. ✅ **DONE (0.0.19)** — **Wire LTL opcodes** to `temporal_ltl::evaluate_ltl_trace`
   (gate semantics: temporal property violated → frame fails). Live-lane tests for
   all 5 operators in `tests/modalities_active.rs`. NOTE the §9.2 predicate-packing
   convention is still UNPINNED — LTL compares the FULL `NQuin.predicate`; the
   ontology compiler must use unpacked predicates for temporal rules or they'll
   silently fail to match. **Pin this before compiling sense/temporal N3 rules.**
2. ✅ **DONE (0.0.19)** — **Wire Allen-interval opcode** to
   `spatio_temporal::evaluate_temporal` (frame regs = the two intervals' bounds).
   Live-lane tests in `tests/modalities_active.rs`. STILL TODO: the **RCC-8 spatial**
   relation has no VM opcode (`evaluate_rcc8` is library-only) — add `NativeRcc8`.
3. **Live-lane smoke tests** — covered in `tests/modalities_active.rs` (14 tests):
   LTL×5, Allen, linear, dialectical, paraconsistent, probabilistic, DL,
   argumentation, defeasible/`NativeUnless`. Still TODO: **epistemic**
   (`NativeEpistemicEval`) and **asp** (`NativeAspStableModels`) need a VM-lane test
   (their evaluators take specific quin shapes — read `epistemic::evaluate_epistemic_frame`
   and `asp::enumerate_stable_models` first).
4. **Remaining unwired** (decide if each needs a deontic-style VM lane): RCC-8
   spatial (`evaluate_rcc8`, pairs with the wired Allen temporal), `diffusion`,
   `graph_theory`, `control_feedback` — these are more computational than
   deontic-style logics.
4. **Audit stub markers**: `spatio_temporal.rs` (5), `dialectical.rs` (2),
   `interval_reasoning.rs` (2), `control_feedback.rs`/`core.rs`/`rules.rs` (1 each)
   — confirm each is real or convert to `PendingImplementation`.
5. **No-test files** to cover: `n3logic.rs` (the CLI agent-intent router — 0 tests),
   and confirm the `*_shacl.rs` vocab files (data-only) need no behavioural tests.

---

## 2. Guard grounding (forward chaining) — follow-ups

Implemented 2026-06-21: `webizen.rs::fire_guard_rules` grounds variable, multi-triple
N3 guards over arena facts (tests in `tests/guard_grounding.rs`). Next:
- **Lift the remaining `agency.n3` guards** through `fire_guard_rules` with corpus
  facts: G1' (done in test), A-duty, A-platform, A-orphan, A-remedy-stripping,
  J-asymmetry, A-sanction, C-duress, C-capacity. Add a fixture + assertion per rule.
- **Ingest path**: route `values.n3`/`agency.n3` so guards register + fire
  automatically via the standard ingest (PLAN §5). Note: ingest currently streams
  `StaticTriple` facts to the `.q42` OUTPUT, NOT the arena — grounding needs facts
  + rules co-resident (bounded working-set arena), so decide the working-set load
  path for the MCP abuse-check / intent-validation use case.
- **Forward-chaining defeasibility**: guard-level `~>` override is not yet modelled
  (deontic-norm `q42:unless` lane handles deontic defeaters; G1 is strict `=>`).
  Decide if any guard needs negation-as-failure / exception-fact semantics.

---

## 3. Zero-knowledge proofs — follow-ups

Core is real + sound as of 0.0.19 (`zk_proofs.rs::ZkProofSystem`,
`deontic_circuit.rs::DeonticAccessCircuit`; round-trip + soundness tests pass).
Remaining:
- **Temporal-window constraint**: `deontic_circuit.rs` binds `temporal_constraint`
  as a public input but does NOT enforce `notBefore ≤ t ≤ notAfter` (the tautology
  was removed, not replaced). Implement a real range check when the access spec is set.
- **Wire `deontic_access` into the vault/access flow** — it's real + tested but not
  yet consumed by the credential-gated subgraph/vault path.
- **`0x01` commitment fallback** (`cryptographic_library.rs`, non-deontic circuit
  ids) is a SHA-256 *binding commitment*, not a ZK proof — branded under system_id
  `"zk_snarks"`. Decide: implement a general circuit→Groth16 path, or rename so it
  doesn't read as a SNARK.
- **Remove unused scaffold**: `zk_proofs.rs::ProvingEngine::generate_proof`
  (`PendingImplementation`) + structural-only `VerificationEngine::verify_proof`
  have no callers — delete or wire.
- **Harden the access circuit**: the `did+role+action = policy_root` additive
  commitment is simplified; Merkle-set membership is the real target.

---

## 4. Browser test suite (`docs/tests/`) — foundational honesty

The GitHub Pages suite (`mediaprophet.github.io/qualiaDB/tests/`) looked near-all-green
but was misleading. Three distinct issues:

- ✅ **FIXED (0.0.19)** — **skip ≡ pass**: the harness scored any test that didn't
  throw as PASS, so daemon-offline (`if (!ctx.native) return`) and missing-WASM-export
  tests counted as passes. Now `test-runner.js` counts assertions; a test that asserts
  nothing is reported **skip**, not pass (+ `runner.skip()`). Browser UI + headless show
  Passed/Skipped/Failed. Honest now: **wasm mode 496 passed / 15 skipped / 3 failed**
  (was read as ~514 "passed"); both mode 498/84/3.

- ⬜ **WASM data-format binding bugs (ENGINE, needs WASM rebuild)** — now visible:
  - `parse_csv_wasm` and `parse_json_mapping_wasm` return structs with u64 quin fields;
    `wasm_bridge.rs` uses plain `serde_wasm_bindgen::to_value(&result)`, which renders
    a u64 > 2^53 as a JS Number and throws "can't be represented as a JavaScript number".
    **Fix:** serialize quin-returning exports with
    `Serializer::new().serialize_large_number_types_as_bigints(true)`, then `wasm-pack`
    rebuild → copy to `docs/playground/`.
  - `parse_json_mapping_wasm` on a JSON **array** (`[{…},{…}]`) returns 0 quins — arrays
    unhandled. Decide: support arrays, or adjust the test if single-object is by design.

- ⬜ **modality-* suites test JS REIMPLEMENTATIONS, not the engine** — e.g.
  `modality-ltl.js` reimplements `evaluate_ltl_trace` in JS and tests that copy. Green =
  "the JS mirror is self-consistent", NOT "the Rust/WASM engine works". Most modalities
  (deontic/LTL/epistemic/…) are NOT in `EXPECTED_WASM_EXPORTS`, so they can't be tested
  from the browser at all. Options: (a) expose the modality evaluators as WASM exports and
  test the real engine; (b) relabel these as "reference-logic self-consistency" suites so
  they don't read as engine verification. The native Rust tests (`deontic_smoke`,
  `guard_grounding`, `modalities_active`) ARE the real engine checks.

## 5. Repo hygiene (low priority)

- **4 stray files** committed in the `0.0.19` checkpoint (`7e8b2566f`):
  `crates/webizen-studio/dx-build.log` ×3 (build logs) + `qpu_bridge.rs.bak`
  (backup of a pre-existing refactor). Drop the logs (gitignore + `git rm --cached`).
- **Residual `qualia.*`** strings remain only in `.claude/worktrees/*` (other agents'
  worktrees) and `target/doc/*` (generated) — not in tracked source; cosmetic.

---

## 6. Pointers
- Reference impl & discipline: `CLAUDE.md`, `AGENTS.md`, `core-ontologies/PLAN.md` §17.
- Live-lane test template: `crates/qualia-core-db/tests/deontic_smoke.rs`.
- Grounding test template: `crates/qualia-core-db/tests/guard_grounding.rs`.
- Predicate-packing hazard (pin before LTL/temporal wiring): PLAN §9.2.
