# Qualia Capability Ledger (auto-generated)

Known-answer verification of *claimed* engine capabilities. `REAL` = produced the
correct answer to an independently-computed problem (not "returned Ok"). Regenerate:
`cargo test -p qualia-core-db --test capability_ledger -- --nocapture`.

| Category | Capability | Verdict | Evidence |
|---|---|---|---|
| Algebra/CAS | symbolic differentiation | REAL    ✓ | d/dx(x^2) evaluated: x=3→6, x=5→10  (== 2x) ✓ |
| Algebra/CAS | symbolic MATRIX algebra | ABSENT  ∅ | CAS Expr is scalar-only (no Matrix/Tensor variant) → no symbolic matrix simplification (the AWQ-fold/tensor-rewrite class needs this) |
| Linear algebra | Matrix4×4 multiply/determinant | PARTIAL ◐ | det(diag 2,3,4,5)=120==120 ✓, A·A diag==[4,9,16,25] ✓ — correct but fixed 4×4 (not general N×N) |
| Linear algebra | general N×N eigen / SVD / tensor | PRESENT ? | FixedLanczosEigensolver / ConstTensorContractor / Tensor3x3x3 exported — present; v2 known-answer probe pending |
| Logic/temporal | LTL trace evaluation | REAL    ✓ | G(all)=T, G(one-differs)=F, F(present)=T, F(absent)=F — genuine temporal eval over the trace ✓ |
| Quantization | ternary BitNet GEMM | REAL    ✓ | scale·Σtrit·act = [-2.0, 10.0] == [-2, 10] ✓ |

**Summary:** 3 REAL · 1 PARTIAL · 0 STUB · 1 ABSENT · 1 PRESENT(unprobed) · 6 total.

_v1 slice. Coverage grows incrementally; v2 adds soundness probes for the documented
fakes (zk commitment-vs-proof, logic.rs threshold-vs-LTL, n3 router-vs-evaluator) + deontic/SHACL._
