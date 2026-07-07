//! Capability-verification ledger — the mechanized agent-honesty guard.
//!
//! The recurring project pathology is "said to be done, later found isn't." This harness converts
//! that fog into an evidence-backed `REAL / PARTIAL / STUB / ABSENT / PRESENT` map by running each
//! *claimed* capability against a **known-answer oracle** — i.e. a problem whose correct answer is
//! computed independently (by hand or a trivially-correct reference). A capability is `REAL` only if
//! it produces the known answer; if a future change breaks it, the check flips to `STUB` and the
//! regression assert fails. This is NOT "the function returned Ok" (a stub returns Ok); it is
//! "the function produced the *correct* result."
//!
//! It is a REPORT (always completes, prints the table, writes the `.md` artifact) plus a small set
//! of regression asserts for the capabilities already verified `REAL`. Coverage grows incrementally;
//! this is the v1 slice (the pieces most relevant to the LLM/AWQ work + a soundness probe).
//!
//! Run: `cargo test -p qualia-core-db --test capability_ledger -- --nocapture`
//!
//! Integration test (no `cfg(test)`) so it exercises the *real* library, not test-capped paths.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::NQuin;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Verdict {
    /// Produced the known answer — verified correct.
    Real,
    /// Correct but narrower than the broad claim (scope-limited).
    Partial,
    /// Present but produced a wrong/placeholder result (a fake caught).
    Stub,
    /// Claimed capability has no implementation to call.
    Absent,
    /// Exists/exported but not yet probed in this slice (honestly unverified).
    Present,
}

impl Verdict {
    fn tag(self) -> &'static str {
        match self {
            Verdict::Real => "REAL    ✓",
            Verdict::Partial => "PARTIAL ◐",
            Verdict::Stub => "STUB    ✗",
            Verdict::Absent => "ABSENT  ∅",
            Verdict::Present => "PRESENT ?",
        }
    }
}

/// One NQuin carrying a given predicate (the LTL trace step the evaluator inspects).
fn mk(pred: u64) -> NQuin {
    NQuin {
        subject: 0,
        predicate: pred,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

// ── Checks (each returns the verdict + the evidence that justifies it) ────────────────────────────

/// Quantization: BitNet ternary GEMM. Known answer (hand-computed): trits [[1,-1,0],[0,1,1]],
/// scale 2.0, act [1,2,3] → [2·(1−2+0), 2·(0+2+3)] = [−2, 10].
fn check_ternary() -> (Verdict, String) {
    use qualia_core_db::ternary::{pack_trits, ternary_gemm_cpu};
    let trits: [i8; 6] = [1, -1, 0, 0, 1, 1];
    let packed = pack_trits(&trits);
    let act = [1.0f32, 2.0, 3.0];
    let mut out = [0.0f32; 2];
    ternary_gemm_cpu(&act, &packed, 2.0, 3, 2, 1, 0, 0, &mut out);
    if (out[0] + 2.0).abs() < 1e-6 && (out[1] - 10.0).abs() < 1e-6 {
        (
            Verdict::Real,
            format!("scale·Σtrit·act = {out:?} == [-2, 10] ✓"),
        )
    } else {
        (Verdict::Stub, format!("expected [-2, 10], got {out:?}"))
    }
}

/// Logic/temporal: LTL over a trace — a SOUNDNESS probe (a fake threshold-LTL would not return
/// false for `Globally` when one step differs). `Globally(p)`: all steps have predicate p.
/// `Finally(p)`: some step has p.
fn check_ltl() -> (Verdict, String) {
    use qualia_core_db::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
    let p = 42u64;
    let all_p = [mk(p), mk(p), mk(p)];
    let one_diff = [mk(p), mk(99), mk(p)];
    let none = [mk(1), mk(2)];
    let g_all = evaluate_ltl_trace(&all_p, &LtlFormula::Globally(p));
    let g_diff = evaluate_ltl_trace(&one_diff, &LtlFormula::Globally(p));
    let f_some = evaluate_ltl_trace(&one_diff, &LtlFormula::Finally(p));
    let f_none = evaluate_ltl_trace(&none, &LtlFormula::Finally(p));
    if g_all && !g_diff && f_some && !f_none {
        (
            Verdict::Real,
            "G(all)=T, G(one-differs)=F, F(present)=T, F(absent)=F — genuine temporal eval over the trace ✓"
                .to_string(),
        )
    } else {
        (
            Verdict::Stub,
            format!("LTL soundness FAILED: G(all)={g_all} G(diff)={g_diff} F(some)={f_some} F(none)={f_none}"),
        )
    }
}

/// Algebra/CAS: symbolic differentiation. Known answer: d/dx(x²) = 2x, verified by evaluating the
/// derivative at sampled points (form-independent — robust to whatever AST `differentiate` emits).
fn check_cas_diff() -> (Verdict, String) {
    use qualia_core_db::specialized_libs::symbolic_algebra::{differentiate, pow, var};
    let x2 = pow(var("x"), 2);
    let d = differentiate(&x2, "x");
    let mut env: HashMap<String, f64> = HashMap::new();
    env.insert("x".to_string(), 3.0);
    let v3 = d.eval(&env);
    env.insert("x".to_string(), 5.0);
    let v5 = d.eval(&env);
    match (v3, v5) {
        (Some(a), Some(b)) if (a - 6.0).abs() < 1e-9 && (b - 10.0).abs() < 1e-9 => (
            Verdict::Real,
            format!("d/dx(x^2) evaluated: x=3→{a}, x=5→{b}  (== 2x) ✓"),
        ),
        _ => (
            Verdict::Stub,
            format!("d/dx(x^2) wrong: x=3→{v3:?}, x=5→{v5:?} (expected 6, 10)"),
        ),
    }
}

/// Algebra/CAS: symbolic MATRIX/tensor algebra. ABSENT — the `Expr` enum is scalar-only
/// (Const/Var/Add/Sub/Mul/Div/Pow/Neg/Sqrt); there is no Matrix/Tensor variant, so the CAS cannot
/// symbolically simplify matrix expressions. This directly bounds the "tensor-graph rewrite / fold
/// S⁻¹ into the layer" proposal: not buildable on the current CAS without a new tensor algebra.
fn check_cas_matrix() -> (Verdict, String) {
    (
        Verdict::Absent,
        "CAS Expr is scalar-only (no Matrix/Tensor variant) → no symbolic matrix simplification (the AWQ-fold/tensor-rewrite class needs this)"
            .to_string(),
    )
}

/// Linear algebra: dense Matrix4×4. Known answer: det(diag(2,3,4,5)) = 120; A·A = diag(4,9,16,25).
/// REAL but scope-limited (fixed 4×4, graphics-oriented) — hence PARTIAL against a general claim.
fn check_linalg_matrix4() -> (Verdict, String) {
    use qualia_core_db::solvers::Matrix4x4;
    let mut m = Matrix4x4::zero();
    m.set(0, 0, 2.0);
    m.set(1, 1, 3.0);
    m.set(2, 2, 4.0);
    m.set(3, 3, 5.0);
    let det = m.determinant();
    let m2 = m.multiply_matrix(&m);
    let ok = (det - 120.0).abs() < 1e-9
        && (m2.get(1, 1) - 9.0).abs() < 1e-9
        && m2.get(0, 1).abs() < 1e-12
        && (m2.get(3, 3) - 25.0).abs() < 1e-9;
    if ok {
        (
            Verdict::Partial,
            format!("det(diag 2,3,4,5)={det}==120 ✓, A·A diag==[4,9,16,25] ✓ — correct but fixed 4×4 (not general N×N)"),
        )
    } else {
        (
            Verdict::Stub,
            format!(
                "Matrix4×4 wrong: det={det} (exp 120), (A·A)[1][1]={} (exp 9)",
                m2.get(1, 1)
            ),
        )
    }
}

/// Linear algebra: general N×N eigen / SVD / tensor contraction. PRESENT (exported:
/// `FixedLanczosEigensolver`, `ConstTensorContractor`, `Tensor3x3x3`) but not probed in this v1
/// slice — scheduled for a v2 known-answer probe (e.g. eigenvalues of a known symmetric matrix).
fn check_eigen_svd() -> (Verdict, String) {
    (
        Verdict::Present,
        "FixedLanczosEigensolver / ConstTensorContractor / Tensor3x3x3 exported — present; v2 known-answer probe pending"
            .to_string(),
    )
}

// ── Runner ────────────────────────────────────────────────────────────────────────────────────

fn ledger_md_path() -> PathBuf {
    for root in [".dev-docs", "../../.dev-docs"] {
        let p = PathBuf::from(root);
        if p.exists() {
            return p.join("CAPABILITY_LEDGER.md");
        }
    }
    PathBuf::from("CAPABILITY_LEDGER.md")
}

#[test]
fn capability_ledger() {
    type Check = fn() -> (Verdict, String);
    // (category, capability, the claim being checked, runner)
    let checks: &[(&str, &str, &str, Check)] = &[
        (
            "Algebra/CAS",
            "symbolic differentiation",
            "d/dx via Expr → correct derivative",
            check_cas_diff,
        ),
        (
            "Algebra/CAS",
            "symbolic MATRIX algebra",
            "matrix/tensor symbolic simplification",
            check_cas_matrix,
        ),
        (
            "Linear algebra",
            "Matrix4×4 multiply/determinant",
            "dense 4×4 ops",
            check_linalg_matrix4,
        ),
        (
            "Linear algebra",
            "general N×N eigen / SVD / tensor",
            "FixedLanczosEigensolver et al.",
            check_eigen_svd,
        ),
        (
            "Logic/temporal",
            "LTL trace evaluation",
            "real temporal operators over a trace",
            check_ltl,
        ),
        (
            "Quantization",
            "ternary BitNet GEMM",
            "scale·Σ trit·act",
            check_ternary,
        ),
    ];

    let mut rows: Vec<(&str, &str, Verdict, String)> = Vec::new();
    for (cat, name, _claim, run) in checks {
        let (v, ev) = run();
        rows.push((cat, name, v, ev));
    }

    // Console table.
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
    eprintln!("QUALIA CAPABILITY LEDGER (v1) — known-answer verification (REAL means proven, not claimed)");
    eprintln!("════════════════════════════════════════════════════════════════════════════════");
    let mut cur = "";
    for (cat, name, v, ev) in &rows {
        if *cat != cur {
            eprintln!("── {cat} ──");
            cur = cat;
        }
        eprintln!("  [{}] {:<34} {}", v.tag(), name, ev);
    }
    let count = |w: Verdict| rows.iter().filter(|(_, _, v, _)| *v == w).count();
    eprintln!("────────────────────────────────────────────────────────────────────────────────");
    eprintln!(
        "summary: {} REAL · {} PARTIAL · {} STUB · {} ABSENT · {} PRESENT(unprobed) · {} total",
        count(Verdict::Real),
        count(Verdict::Partial),
        count(Verdict::Stub),
        count(Verdict::Absent),
        count(Verdict::Present),
        rows.len()
    );
    eprintln!("════════════════════════════════════════════════════════════════════════════════");

    // Markdown artifact (gitignored .dev-docs — the human-facing ground-truth map).
    let mut md = String::new();
    md.push_str("# Qualia Capability Ledger (auto-generated)\n\n");
    md.push_str(
        "Known-answer verification of *claimed* engine capabilities. `REAL` = produced the\n",
    );
    md.push_str(
        "correct answer to an independently-computed problem (not \"returned Ok\"). Regenerate:\n",
    );
    md.push_str("`cargo test -p qualia-core-db --test capability_ledger -- --nocapture`.\n\n");
    md.push_str("| Category | Capability | Verdict | Evidence |\n|---|---|---|---|\n");
    for (cat, name, v, ev) in &rows {
        md.push_str(&format!("| {cat} | {name} | {} | {ev} |\n", v.tag().trim()));
    }
    md.push_str(&format!(
        "\n**Summary:** {} REAL · {} PARTIAL · {} STUB · {} ABSENT · {} PRESENT(unprobed) · {} total.\n",
        count(Verdict::Real),
        count(Verdict::Partial),
        count(Verdict::Stub),
        count(Verdict::Absent),
        count(Verdict::Present),
        rows.len()
    ));
    md.push_str(
        "\n_v1 slice. Coverage grows incrementally; v2 adds soundness probes for the documented\n",
    );
    md.push_str("fakes (zk commitment-vs-proof, logic.rs threshold-vs-LTL, n3 router-vs-evaluator) + deontic/SHACL._\n");
    let path = ledger_md_path();
    if let Err(e) = std::fs::write(&path, &md) {
        eprintln!("[ledger] could not write {}: {e}", path.display());
    } else {
        eprintln!("[ledger] wrote {}", path.display());
    }

    // Regression guard: capabilities already verified REAL must not silently rot to STUB.
    // (PARTIAL/ABSENT/PRESENT are honest states, not failures.)
    let must_be_real = [
        "ternary BitNet GEMM",
        "LTL trace evaluation",
        "symbolic differentiation",
    ];
    for (_, name, v, ev) in &rows {
        if must_be_real.contains(name) {
            assert_ne!(
                *v,
                Verdict::Stub,
                "REGRESSION: '{name}' verified REAL before, now STUB: {ev}"
            );
            assert_eq!(*v, Verdict::Real, "'{name}' must stay REAL ({ev})");
        }
    }
    assert_eq!(
        rows.len(),
        checks.len(),
        "ledger framework must produce one row per check"
    );
}
