//! Result verification — the trusted gate that runs *before* payment. It never trusts
//! the executor; it re-derives correctness cheaply with a local reference.
//!
//! * **Dense matrix product** → **Freivalds' algorithm**: to check `A·B = C` without
//!   recomputing the `O(n³)` product, pick a random ±1 vector `x` and test
//!   `A(Bx) == Cx` in `O(n²)`. A wrong `C` passes a single round with probability ≤ ½,
//!   so `r` independent rounds bound the false-accept probability by `2⁻ʳ`. This makes
//!   verify-before-pay *cheaper than doing the work* — essential for the economics.
//! * **Embedding artifact** → **ranking reproduction**: a trained table is trusted only
//!   if it actually ranks the held-out check triples well (MRR ≥ floor) — termination is
//!   not evidence of learning.

use super::job::{JobInput, JobResult};
use crate::solvers::linear_algebra::gemm::{matvec, Transpose};
use crate::solvers::optimization::metaheuristics::Rng;

/// The verdict of verifying a returned result against the job's input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerificationVerdict {
    /// Result re-derives correctly; `confidence` ∈ (0,1] (1 − false-accept bound).
    Verified { confidence: f64 },
    /// Result does not match the trusted reference — do not pay.
    Rejected { reason: &'static str },
}

impl VerificationVerdict {
    pub fn is_verified(&self) -> bool {
        matches!(self, VerificationVerdict::Verified { .. })
    }
}

/// Tunables for verification.
#[derive(Debug, Clone, Copy)]
pub struct VerifyPolicy {
    /// Freivalds rounds (false-accept ≤ 2⁻ʳ). More rounds = more confidence.
    pub freivalds_rounds: usize,
    /// Numeric tolerance for the product check (floating-point slack).
    pub tol: f64,
    /// RNG seed for the random projection vectors (deterministic, auditable).
    pub seed: u64,
    /// Minimum MRR an embedding artifact must achieve on the check set to be accepted.
    pub min_mrr: f64,
}

impl Default for VerifyPolicy {
    fn default() -> Self {
        Self {
            freivalds_rounds: 16,
            tol: 1e-6,
            seed: 0xF1E1,
            min_mrr: 0.9,
        }
    }
}

/// Verify a result against the job input. Mismatched kinds are rejected (fail closed).
pub fn verify(input: &JobInput, result: &JobResult, policy: VerifyPolicy) -> VerificationVerdict {
    match (input, result) {
        (JobInput::DenseLinearProduct { m, k, n, a, b }, JobResult::DenseLinearProduct { c }) => {
            if c.len() != m * n {
                return VerificationVerdict::Rejected {
                    reason: "result has wrong dimensions",
                };
            }
            verify_product(*m, *k, *n, a, b, c, policy)
        }
        (
            JobInput::EmbeddingArtifact {
                n_entities, check, ..
            },
            JobResult::EmbeddingArtifact { table },
        ) => verify_artifact(table, check, *n_entities, policy.min_mrr),
        _ => VerificationVerdict::Rejected {
            reason: "result kind does not match job kind",
        },
    }
}

/// Freivalds' check that `C == A·B`. `A` is `m×k`, `B` is `k×n`, `C` is `m×n`.
fn verify_product(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
    c: &[f64],
    policy: VerifyPolicy,
) -> VerificationVerdict {
    let mut rng = Rng(policy.seed ^ 0x5A17_C0DE);
    let mut x = vec![0.0; n];
    let mut bx = vec![0.0; k];
    let mut abx = vec![0.0; m];
    let mut cx = vec![0.0; m];
    for _ in 0..policy.freivalds_rounds.max(1) {
        // Random ±1 projection vector.
        for xi in x.iter_mut() {
            *xi = if rng.unit() < 0.5 { -1.0 } else { 1.0 };
        }
        // bx = B·x  (B is k×n)
        if matvec(Transpose::No, k, n, b, &x, &mut bx).is_err() {
            return VerificationVerdict::Rejected {
                reason: "reference matvec failed",
            };
        }
        // abx = A·(Bx)  (A is m×k)
        if matvec(Transpose::No, m, k, a, &bx, &mut abx).is_err() {
            return VerificationVerdict::Rejected {
                reason: "reference matvec failed",
            };
        }
        // cx = C·x  (C is m×n)
        if matvec(Transpose::No, m, n, c, &x, &mut cx).is_err() {
            return VerificationVerdict::Rejected {
                reason: "reference matvec failed",
            };
        }
        for i in 0..m {
            if (abx[i] - cx[i]).abs() > policy.tol {
                return VerificationVerdict::Rejected {
                    reason: "A·B ≠ C (Freivalds)",
                };
            }
        }
    }
    let confidence = 1.0 - 2.0_f64.powi(-(policy.freivalds_rounds.max(1) as i32));
    VerificationVerdict::Verified { confidence }
}

/// Verify a trained embedding table reproduces good ranking on the held-out checks.
///
/// Uses a **pessimistic** rank (ties counted *against* the true tail): a degenerate
/// table that scores every candidate equally — e.g. an all-zero "trained nothing"
/// table — then ranks the true tail last, scoring near-zero MRR and being rejected.
/// (Optimistic tie-breaking would let such a table masquerade as perfect.)
fn verify_artifact(
    table: &crate::solvers::learning::kg_embedding::EmbeddingTable,
    check: &[(usize, usize, usize)],
    n_entities: usize,
    min_mrr: f64,
) -> VerificationVerdict {
    if check.is_empty() {
        return VerificationVerdict::Rejected {
            reason: "no check set to verify the artifact",
        };
    }
    let mut recip_sum = 0.0;
    for &(h, r, t) in check {
        let target = match table.score(h, r, t) {
            Ok(s) => s,
            Err(_) => {
                return VerificationVerdict::Rejected {
                    reason: "artifact could not be scored",
                };
            }
        };
        // Pessimistic rank: 1 + #{ other candidates scoring ≥ the true tail }.
        let mut rank = 1usize;
        for c in 0..n_entities {
            if c == t {
                continue;
            }
            match table.score(h, r, c) {
                Ok(s) if s >= target => rank += 1,
                Ok(_) => {}
                Err(_) => {
                    return VerificationVerdict::Rejected {
                        reason: "artifact could not be scored",
                    };
                }
            }
        }
        recip_sum += 1.0 / rank as f64;
    }
    let mrr = recip_sum / check.len() as f64;
    if mrr >= min_mrr {
        VerificationVerdict::Verified { confidence: mrr }
    } else {
        VerificationVerdict::Rejected {
            reason: "artifact MRR below the acceptance floor",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::kg_embedding::{train, ScoreModel, TrainConfig};

    #[test]
    fn freivalds_accepts_a_correct_product() {
        // [[1,2],[3,4]]·[[5,6],[7,8]] = [[19,22],[43,50]]
        let input = JobInput::DenseLinearProduct {
            m: 2,
            k: 2,
            n: 2,
            a: vec![1.0, 2.0, 3.0, 4.0],
            b: vec![5.0, 6.0, 7.0, 8.0],
        };
        let good = JobResult::DenseLinearProduct {
            c: vec![19.0, 22.0, 43.0, 50.0],
        };
        assert!(verify(&input, &good, VerifyPolicy::default()).is_verified());
    }

    #[test]
    fn freivalds_rejects_a_wrong_product() {
        let input = JobInput::DenseLinearProduct {
            m: 2,
            k: 2,
            n: 2,
            a: vec![1.0, 2.0, 3.0, 4.0],
            b: vec![5.0, 6.0, 7.0, 8.0],
        };
        // One entry corrupted.
        let bad = JobResult::DenseLinearProduct {
            c: vec![19.0, 22.0, 43.0, 999.0],
        };
        assert!(!verify(&input, &bad, VerifyPolicy::default()).is_verified());
    }

    #[test]
    fn freivalds_rejects_a_subtly_wrong_product() {
        // A single-element error of 1.0 — must still be caught with high probability.
        let n = 6;
        let a: Vec<f64> = (0..n * n).map(|i| (i % 7) as f64).collect();
        let b: Vec<f64> = (0..n * n).map(|i| (i % 5) as f64 - 2.0).collect();
        let mut c = vec![0.0; n * n];
        crate::solvers::linear_algebra::gemm::matmul(n, n, n, &a, &b, &mut c).unwrap();
        let input = JobInput::DenseLinearProduct {
            m: n,
            k: n,
            n,
            a,
            b,
        };
        c[10] += 1.0; // corrupt one cell
        let bad = JobResult::DenseLinearProduct { c };
        assert!(!verify(&input, &bad, VerifyPolicy::default()).is_verified());
    }

    #[test]
    fn artifact_verification_accepts_a_learned_table_and_rejects_garbage() {
        let triples = vec![(0, 0, 1), (2, 0, 3)];
        let cfg = TrainConfig {
            model: ScoreModel::TransE { p: 2 },
            rank: 8,
            epochs: 400,
            lr: 0.05,
            margin: 1.0,
            reg: 0.0,
            neg_per_pos: 4,
            seed: 7,
        };
        let table = train(&triples, 4, 1, cfg).unwrap();
        let input = JobInput::EmbeddingArtifact {
            triples,
            n_entities: 4,
            n_relations: 1,
            cfg,
            check: vec![(0, 0, 1), (2, 0, 3)],
        };
        let good = JobResult::EmbeddingArtifact { table };
        assert!(verify(&input, &good, VerifyPolicy::default()).is_verified());

        // An untrained (zeroed) table cannot reproduce ranking → rejected.
        let empty = crate::solvers::learning::kg_embedding::EmbeddingTable::zeros(
            ScoreModel::TransE { p: 2 },
            8,
            4,
            1,
        )
        .unwrap();
        let garbage = JobResult::EmbeddingArtifact { table: empty };
        assert!(!verify(&input, &garbage, VerifyPolicy::default()).is_verified());
    }

    #[test]
    fn mismatched_kinds_rejected() {
        let input = JobInput::DenseLinearProduct {
            m: 1,
            k: 1,
            n: 1,
            a: vec![1.0],
            b: vec![1.0],
        };
        let table = crate::solvers::learning::kg_embedding::EmbeddingTable::zeros(
            ScoreModel::TransE { p: 2 },
            2,
            2,
            1,
        )
        .unwrap();
        let wrong = JobResult::EmbeddingArtifact { table };
        assert!(!verify(&input, &wrong, VerifyPolicy::default()).is_verified());
    }
}
