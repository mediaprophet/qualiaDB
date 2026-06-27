//! Job executors. The [`JobExecutor`] trait is what a worker cell — local or a remote
//! peer — implements. [`LocalKernelExecutor`] is the **real** one: it runs each job's
//! kernel through the actual engine (the dynamic GEMM and the real KGE trainer), so a
//! dispatched job produces genuine computed work — never a stub.
//!
//! In the distributed setting the executor is *untrusted* (it could be a stranger's
//! solar node). That is exactly why dispatch always follows execution with independent
//! verification ([`super::verify`]) before any payment.

use super::job::{JobInput, JobResult};
use super::SwarmError;
use crate::solvers::learning::kg_embedding::{train, EmbeddingTable};
use crate::solvers::linear_algebra::gemm::matmul;

/// A node that can execute swarm jobs. Object-safe so it can be boxed and swapped
/// (a local cell, a remote peer proxy, a test double).
pub trait JobExecutor {
    fn execute(&self, input: &JobInput) -> Result<JobResult, SwarmError>;
}

/// Executes jobs on the local CPU through the real engine kernels.
pub struct LocalKernelExecutor;

impl JobExecutor for LocalKernelExecutor {
    fn execute(&self, input: &JobInput) -> Result<JobResult, SwarmError> {
        if !input.is_well_formed() {
            return Err(SwarmError::InvalidJob);
        }
        match input {
            JobInput::DenseLinearProduct { m, k, n, a, b } => {
                let mut c = vec![0.0; m * n];
                matmul(*m, *k, *n, a, b, &mut c).map_err(|_| SwarmError::KernelFailed)?;
                Ok(JobResult::DenseLinearProduct { c })
            }
            JobInput::EmbeddingArtifact { triples, n_entities, n_relations, cfg, .. } => {
                let table: EmbeddingTable = train(triples, *n_entities, *n_relations, *cfg)
                    .map_err(|_| SwarmError::KernelFailed)?;
                Ok(JobResult::EmbeddingArtifact { table })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::kg_embedding::{ScoreModel, TrainConfig};

    #[test]
    fn local_executor_computes_a_real_product() {
        let input = JobInput::DenseLinearProduct {
            m: 2,
            k: 2,
            n: 2,
            a: vec![1.0, 2.0, 3.0, 4.0],
            b: vec![5.0, 6.0, 7.0, 8.0],
        };
        let r = LocalKernelExecutor.execute(&input).unwrap();
        match r {
            // [[1,2],[3,4]]·[[5,6],[7,8]] = [[19,22],[43,50]]
            JobResult::DenseLinearProduct { c } => {
                assert_eq!(c, vec![19.0, 22.0, 43.0, 50.0]);
            }
            _ => panic!("wrong result kind"),
        }
    }

    #[test]
    fn local_executor_trains_a_real_artifact() {
        let input = JobInput::EmbeddingArtifact {
            triples: vec![(0, 0, 1), (2, 0, 3)],
            n_entities: 4,
            n_relations: 1,
            cfg: TrainConfig {
                model: ScoreModel::TransE { p: 2 },
                rank: 8,
                epochs: 200,
                lr: 0.05,
                margin: 1.0,
                reg: 0.0,
                neg_per_pos: 4,
                seed: 7,
            },
            check: vec![(0, 0, 1)],
        };
        let r = LocalKernelExecutor.execute(&input).unwrap();
        match r {
            JobResult::EmbeddingArtifact { table } => {
                // The trained table scores the true tail above a wrong one.
                assert!(table.score(0, 0, 1).unwrap() > table.score(0, 0, 3).unwrap());
            }
            _ => panic!("wrong result kind"),
        }
    }

    #[test]
    fn malformed_job_fails_closed() {
        let bad = JobInput::DenseLinearProduct { m: 2, k: 2, n: 2, a: vec![1.0], b: vec![1.0; 4] };
        assert_eq!(LocalKernelExecutor.execute(&bad).unwrap_err(), SwarmError::InvalidJob);
    }
}
