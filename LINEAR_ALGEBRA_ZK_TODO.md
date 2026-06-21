# TODO — Linear algebra, the 10D manifold, and ZK-private computation

> Status note (2026-06-21): the immediate crash is **fixed** — `private_matrix_multiply`
> and `ZkProofSystem::generate_semantic_proof` no longer panic with
> `MalformedVerifyingKey`. But that fix only made the Groth16 *plumbing* consistent;
> the items below are the substantive work that remains. Timothy flagged that algebra
> generally (matrices, quadratics, the wgpu 10D manifold) is foundational and must not
> be forgotten. This file is the durable record.

## 1. What was actually wrong (and what the fix did / didn't do)

`crates/qualia-core-db/src/zk_proofs.rs`

- **Fixed:** `extract_public_inputs` hardcoded **1** public input while
  `build_function_circuit` declared **0** (every variable added as `Private`). Groth16
  setup therefore built `vk.gamma_abc_g1` for 0 public inputs, and verify was handed 1
  → `SynthesisError::MalformedVerifyingKey`. `extract_public_inputs(circuit_id, witness)`
  now derives the public inputs from the circuit's **actual** declared public-input
  variables, in order, reading each value from the same witness the prover uses. Counts
  and assignments now agree by construction.

- **NOT fixed (the real depth):** the statement circuits are **structural placeholders**.
  `build_function_circuit` / `build_optimization_circuit` add variables but **no
  constraints**; `build_equality_circuit` / `build_inequality_circuit` add a single
  constraint over variable names (`"left"`/`"right"`/`"result"`) that were never added to
  the circuit. So:
  - The proof produced for `private_matrix_multiply` is a satisfiability proof of an
    essentially **empty circuit**. It does **not** cryptographically bind the proof to the
    actual A·B = C computation.
  - `LinearAlgebraResult.privacy_preserved = true` therefore **overstates** the
    guarantee. Today it means "a well-formed Groth16 proof was produced and verified",
    not "the multiplication was proven in zero knowledge".

  **Action:** build a real R1CS circuit for matrix multiply (and for the other statement
  types): allocate A, B as private witnesses, C's entries as public inputs (or committed),
  and enforce `C[i][j] = Σ_k A[i][k]·B[k][j]` via `enforce_constraint`. The
  `DynamicCircuit` evaluator already supports `Mul`/`Add`/`Neg` expression trees, so this
  can be expressed without new SNARK machinery — it needs the circuit *builders* to emit
  the constraints. Until then, gate or rename `privacy_preserved` so it doesn't claim more
  than is true (honesty-guard: see `agent-accountability.n3`).

## 2. Algebra coverage to build out

`crates/qualia-core-db/src/specialized_libs/linear_algebra.rs` is the home. Real and
working today: matrix create/multiply, linear-system solve (the `solve` test passes).
Wanted, per Timothy ("algebra generally … quadratic equations … is incredibly important"):

- [ ] **Quadratic / polynomial equations** — roots, discriminant, factoring; expose via MCP.
- [ ] **Eigenvalues / eigenvectors, SVD, determinant, inverse, rank** — the standard kit.
- [ ] **Symbolic algebra path** (not just numeric) so equations can be manipulated, not
      only evaluated — ties into the neuro-symbolic tokenizer→ontology binding in
      `STELLAR_MISSION.md`.
- [ ] Each operation should have (a) a numeric backend, (b) an MCP tool, and (c) an
      optional ZK-private variant once §1 is real.

## 3. The wgpu 10D manifold

`crates/qualia-core-db/src/tensor/bake_pipeline.rs` + the wgpu compute path
(`compute_universe.rs`, the `fused_*` shaders).

- The `bake_quin_to_tensor` → `Tensor10D` pipeline and the wgpu device path need more
  work. Note `compute_universe::producer_cycle_with_global_substrate` currently panics
  inside `wgpu-0.19.4` device code on machines without a usable GPU (a separate,
  pre-existing, environment-dependent failure — *not* caused by the algebra/ZK work).
- The 10D manifold encoding shares the NQuin metadata field (the bake `t` clock lives at
  metadata `[32..60]`); any change there must respect the FrameLayout ABI
  (`frame_layout.rs`) — see the role-keyed-overlay rules so the clock doesn't collide
  with quin_type / flags / sensitivity.

## 4. Verification status (this change)

`cargo test -p qualia-core-db --lib` — `zk_proofs` (5), `specialized_libs::linear_algebra`
(7), `semantic_culler` (3) all pass. The one remaining lib failure,
`compute_universe::producer_cycle_with_global_substrate`, is the pre-existing wgpu GPU
panic in §3, unrelated to this work.
