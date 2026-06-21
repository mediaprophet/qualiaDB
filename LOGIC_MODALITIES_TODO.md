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
| **temporal LTL** | `temporal_ltl.rs` (`evaluate_ltl_trace`) | 5 | `NativeLtlGlobally/Finally/Next/Until/Release` | **NO-OP (vm_log only)** | ⚠ **wire opcodes to `evaluate_ltl_trace`** |
| **Allen interval / RCC-8** | `spatio_temporal.rs`, `interval_reasoning.rs` | 3 + 6 | `NativeAllenInterval(u8)` | **NO-OP (vm_log only)** | ⚠ **wire opcode to interval algebra**; review 5+2 stub markers |
| **probabilistic** | `probabilistic.rs` | 1 | none | — | decide: needs a VM lane? |
| **description logic (DL)** | `dl.rs` | 1 | none | — | decide: needs a VM lane? |
| **argumentation** | `argumentation.rs` | 5 | none | — | decide: needs a VM lane? |
| **diffusion** | `diffusion.rs` | 1 | none | — | decide: needs a VM lane? (`qualia:diffuse` block) |
| **graph theory** | `graph_theory.rs` | 7 | none | — | library-only; fine? |
| **control feedback** | `control_feedback.rs` | 4 | none | — | review 1 stub marker |

### Highest-value items (§1)
1. **Wire LTL opcodes** — `NativeLtlGlobally/Finally/Next/Until/Release` currently
   only `vm_log!`. Connect to `temporal_ltl::evaluate_ltl_trace` (the real,
   tested evaluator) so temporal rules actually evaluate in the live lane. **Pin
   the §9.2 predicate-packing convention first** (`temporal_ltl.rs` compares the
   FULL `NQuin.predicate`; a packed property-path hash will silently fail to match).
2. **Wire Allen-interval opcode** — `NativeAllenInterval(mode)` is a no-op; connect
   to `spatio_temporal.rs` / `interval_reasoning.rs`.
3. **Live-lane smoke tests** for each already-wired modality (epistemic, linear,
   asp, paraconsistent, defeasible) mirroring `deontic_smoke.rs` — prove the
   `Native*Eval` opcode produces the asserted verdict, not just that the library
   passes unit tests.
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

## 4. Repo hygiene (low priority)

- **4 stray files** committed in the `0.0.19` checkpoint (`7e8b2566f`):
  `crates/webizen-studio/dx-build.log` ×3 (build logs) + `qpu_bridge.rs.bak`
  (backup of a pre-existing refactor). Drop the logs (gitignore + `git rm --cached`).
- **Residual `qualia.*`** strings remain only in `.claude/worktrees/*` (other agents'
  worktrees) and `target/doc/*` (generated) — not in tracked source; cosmetic.

---

## 5. Pointers
- Reference impl & discipline: `CLAUDE.md`, `AGENTS.md`, `core-ontologies/PLAN.md` §17.
- Live-lane test template: `crates/qualia-core-db/tests/deontic_smoke.rs`.
- Grounding test template: `crates/qualia-core-db/tests/guard_grounding.rs`.
- Predicate-packing hazard (pin before LTL/temporal wiring): PLAN §9.2.
