# TODO — Linear algebra, the 10D manifold, and ZK-private computation

> Status note (2026-06-21): **§1 is now DONE** — `private_matrix_multiply` is backed by
> a real R1CS circuit that cryptographically attests `A·B = C` in zero knowledge
> (round-trip + soundness tests pass). The earlier `MalformedVerifyingKey` crash is also
> fixed. What remains is §2 (broader algebra coverage) and §3 (the wgpu 10D manifold),
> plus making the *generic* statement-circuit builders real for the other statement
> types. Timothy flagged that algebra generally (matrices, quadratics, the wgpu 10D
> manifold) is foundational and must not be forgotten. This file is the durable record.

## 1. What was actually wrong (and what the fix did / didn't do)

`crates/qualia-core-db/src/zk_proofs.rs`

- **Fixed:** `extract_public_inputs` hardcoded **1** public input while
  `build_function_circuit` declared **0** (every variable added as `Private`). Groth16
  setup therefore built `vk.gamma_abc_g1` for 0 public inputs, and verify was handed 1
  → `SynthesisError::MalformedVerifyingKey`. `extract_public_inputs(circuit_id, witness)`
  now derives the public inputs from the circuit's **actual** declared public-input
  variables, in order, reading each value from the same witness the prover uses. Counts
  and assignments now agree by construction.

- **DONE — matrix multiply is now a real ZK proof.** `ZkProofSystem::prove_matrix_multiply`
  (`zk_proofs.rs`) builds an actual R1CS circuit: each `A[i][k]` / `B[k][j]` is a private
  witness, each result entry `C[i][j]` is a public input, and the circuit enforces
  `Σ_k A[i][k]·B[k][j] = C[i][j]` (inner products become intermediate witness variables
  with their own multiplication constraints). A Groth16 proof is generated and verified;
  `private_matrix_multiply` sets `privacy_preserved` only when that proof verifies, and
  returns exactly the attested product. Signed integers are encoded into the field via
  `arkworks_groth16::i128_to_field_element`.
  - Tests: `zk_proofs::tests::test_matrix_multiply_zk_roundtrip` (accepts the true
    product), `test_matrix_multiply_circuit_rejects_false_product` (a falsified
    dot-product result fails to verify — soundness for the sum-of-products construction),
    and `linear_algebra` round-trip + rectangular/negative cases.
  - **Limitation (documented, not hidden):** the ZK circuit operates over integers —
    entries are rounded to the nearest integer (exact for integer / fixed-point matrices,
    the intended use). A fixed-point or field-native fractional encoding is future work.

- **Still placeholders (other statement types).** The GENERIC statement circuits remain
  structural stubs: `build_function_circuit` / `build_optimization_circuit` add variables
  but no constraints; `build_equality_circuit` / `build_inequality_circuit` add a
  constraint over variable names (`"left"`/`"right"`/`"result"`) that were never added.
  `generate_semantic_proof` for those `StatementType`s therefore still proves an
  essentially empty circuit. Matrix multiply no longer routes through that path, but the
  other types should get real constraint emission before their `privacy_preserved`/
  attestation is trusted. (Honesty-guard: see `agent-accountability.n3`.)

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
  work. (Fixed 2026-06-21: `compute_universe::producer_cycle_with_global_substrate` was
  panicking with a wgpu **validation error** — `count_buf` in `tensor/volume_gpu.rs` was
  written via `queue.write_buffer` but created without `COPY_DST`. This was a real code
  bug, NOT "no GPU"; the missing usage flag was added. The broader manifold work remains.)
- The 10D manifold encoding shares the NQuin metadata field (the bake `t` clock lives at
  metadata `[32..60]`); any change there must respect the FrameLayout ABI
  (`frame_layout.rs`) — see the role-keyed-overlay rules so the clock doesn't collide
  with quin_type / flags / sensitivity.

## 4. Verification status (this change)

`cargo test -p qualia-core-db --lib` — **990 passed, 0 failed**. Includes `zk_proofs`
(now with the real matrix-multiply round-trip + soundness tests),
`specialized_libs::linear_algebra`, `semantic_culler`, and
`compute_universe::producer_cycle_with_global_substrate` (the wgpu COPY_DST fix in §3).
