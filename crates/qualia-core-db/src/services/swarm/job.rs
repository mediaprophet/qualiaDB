//! The job envelope: a deterministic, content-addressed specification of work plus
//! its result. "Deterministic" matters — a job is reproducible from its `(kind, input,
//! seed)`, so any node can re-derive or verify it, and a paid job's outcome is
//! auditable rather than taken on trust.

use crate::solvers::learning::kg_embedding::{EmbeddingTable, TrainConfig};

/// The kind of work a job carries. Each maps to a kernel-class with a CPU reference
/// (§13) and a verification strategy ([`super::verify`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Dense matrix product `C = A·B` (kernel-class `DenseLinear`). Verified by
    /// Freivalds' algorithm.
    DenseLinearProduct,
    /// Train a knowledge-graph embedding table (the heavy, run-once, affordability-
    /// gated artifact). Verified by ranking reproduction on a held-out check set.
    EmbeddingArtifact,
}

/// How a job is dispatched across the socially-defined network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobMode {
    /// The principal's own devices cooperate. No payment.
    Personal,
    /// Done with named peers (DID hashes). No payment.
    Collaborative { peers: Vec<u64> },
    /// Dispatched to a provider for payment — the solar-excess case.
    Paid {
        requester_did: u64,
        provider_did: u64,
        /// Agreed price in abstract minor units (µ-units), settled only on `Verified`.
        price_micro_units: u64,
    },
}

/// The actual work payload. Carries the real inputs (content-addressed into the job
/// id), so an executor has everything it needs and a verifier can re-derive the answer.
#[derive(Debug, Clone, PartialEq)]
pub enum JobInput {
    /// `A` is `m×k`, `B` is `k×n`, both row-major.
    DenseLinearProduct {
        m: usize,
        k: usize,
        n: usize,
        a: Vec<f64>,
        b: Vec<f64>,
    },
    /// Train an embedding over `triples`; `check` is the held-out set used to verify
    /// the returned table actually learned (not just terminated).
    EmbeddingArtifact {
        triples: Vec<(usize, usize, usize)>,
        n_entities: usize,
        n_relations: usize,
        cfg: TrainConfig,
        check: Vec<(usize, usize, usize)>,
    },
}

impl JobInput {
    pub fn kind(&self) -> JobKind {
        match self {
            JobInput::DenseLinearProduct { .. } => JobKind::DenseLinearProduct,
            JobInput::EmbeddingArtifact { .. } => JobKind::EmbeddingArtifact,
        }
    }

    /// True if the input is internally well-formed (dimensions consistent).
    pub fn is_well_formed(&self) -> bool {
        match self {
            JobInput::DenseLinearProduct { m, k, n, a, b } => {
                *m > 0 && *k > 0 && *n > 0 && a.len() == m * k && b.len() == k * n
            }
            JobInput::EmbeddingArtifact {
                triples,
                n_entities,
                n_relations,
                ..
            } => {
                !triples.is_empty()
                    && *n_entities > 0
                    && *n_relations > 0
                    && triples
                        .iter()
                        .all(|&(h, r, t)| h < *n_entities && t < *n_entities && r < *n_relations)
            }
        }
    }
}

/// The result an executor returns.
#[derive(Debug, Clone, PartialEq)]
pub enum JobResult {
    DenseLinearProduct { c: Vec<f64> },
    EmbeddingArtifact { table: EmbeddingTable },
}

impl JobResult {
    pub fn kind(&self) -> JobKind {
        match self {
            JobResult::DenseLinearProduct { .. } => JobKind::DenseLinearProduct,
            JobResult::EmbeddingArtifact { .. } => JobKind::EmbeddingArtifact,
        }
    }
}

/// A full job: a content-addressed id, its dispatch mode, and the work payload.
#[derive(Debug, Clone, PartialEq)]
pub struct JobSpec {
    /// Content id — a deterministic hash of `input`. Two identical jobs share an id.
    pub id: u64,
    pub mode: JobMode,
    pub input: JobInput,
}

impl JobSpec {
    pub fn new(mode: JobMode, input: JobInput) -> Self {
        let id = content_id(&input);
        Self { id, mode, input }
    }

    pub fn kind(&self) -> JobKind {
        self.input.kind()
    }
}

/// Deterministic content id (FNV-1a over the structural bytes of the input). Stable
/// across nodes — the basis for content-addressed dispatch and dedup.
pub fn content_id(input: &JobInput) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut byte = |b: u8, h: &mut u64| {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    };
    let mut word = |w: u64, h: &mut u64| {
        for b in w.to_le_bytes() {
            byte(b, h);
        }
    };
    match input {
        JobInput::DenseLinearProduct { m, k, n, a, b } => {
            byte(0x01, &mut h);
            word(*m as u64, &mut h);
            word(*k as u64, &mut h);
            word(*n as u64, &mut h);
            for &v in a.iter().chain(b.iter()) {
                word(v.to_bits(), &mut h);
            }
        }
        JobInput::EmbeddingArtifact {
            triples,
            n_entities,
            n_relations,
            cfg,
            check,
        } => {
            byte(0x02, &mut h);
            word(*n_entities as u64, &mut h);
            word(*n_relations as u64, &mut h);
            word(cfg.seed, &mut h);
            word(cfg.rank as u64, &mut h);
            word(cfg.epochs as u64, &mut h);
            for &(a, b, c) in triples.iter().chain(check.iter()) {
                word(a as u64, &mut h);
                word(b as u64, &mut h);
                word(c as u64, &mut h);
            }
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dense() -> JobInput {
        JobInput::DenseLinearProduct {
            m: 2,
            k: 2,
            n: 2,
            a: vec![1.0, 2.0, 3.0, 4.0],
            b: vec![1.0, 0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn content_id_is_deterministic_and_input_sensitive() {
        let a = content_id(&dense());
        let b = content_id(&dense());
        assert_eq!(a, b, "same input → same id");
        let mut other = dense();
        if let JobInput::DenseLinearProduct { a, .. } = &mut other {
            a[0] = 9.0;
        }
        assert_ne!(content_id(&other), a, "different input → different id");
    }

    #[test]
    fn well_formedness_catches_bad_dims() {
        assert!(dense().is_well_formed());
        let bad = JobInput::DenseLinearProduct {
            m: 2,
            k: 2,
            n: 2,
            a: vec![1.0],
            b: vec![1.0; 4],
        };
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn spec_carries_kind_and_id() {
        let spec = JobSpec::new(JobMode::Personal, dense());
        assert_eq!(spec.kind(), JobKind::DenseLinearProduct);
        assert_eq!(spec.id, content_id(&dense()));
    }
}
