//! Dispatch ties the pieces into the one path that matters: **execute → verify →
//! settle**, with payment impossible without verification.

use super::executor::JobExecutor;
use super::job::{JobMode, JobResult, JobSpec};
use super::settlement::{Escrow, SettlementOutcome};
use super::verify::{verify, VerificationVerdict, VerifyPolicy};
use super::SwarmError;

/// The full outcome of running a swarm job.
#[derive(Debug, Clone)]
pub struct DispatchOutcome {
    /// The computed result. Retained regardless of verdict so a caller can inspect a
    /// rejected result — but a rejected result never yields a payment.
    pub result: JobResult,
    pub verdict: VerificationVerdict,
    /// Present only for `Paid` jobs: the settlement (a payment instruction or a refund).
    /// `None` for `Personal`/`Collaborative` jobs (no money involved).
    pub settlement: Option<SettlementOutcome>,
}

/// Run a job: execute it on `executor`, independently verify the result, and — for a
/// `Paid` job — settle the supplied `escrow` by the verdict. The escrow must already be
/// `Held` (funds committed) for a paid job; if it is missing for a paid job, the job is
/// still executed and verified but no settlement is produced (a caller error surfaced as
/// `None`, never an unguarded payment).
///
/// **Invariant:** a provider payment instruction is emitted only when verification
/// returns `Verified`. There is no code path from a `Rejected` verdict to a `Pay`.
pub fn run_job(
    spec: &JobSpec,
    executor: &dyn JobExecutor,
    policy: VerifyPolicy,
    escrow: Option<&mut Escrow>,
) -> Result<DispatchOutcome, SwarmError> {
    if !spec.input.is_well_formed() {
        return Err(SwarmError::InvalidJob);
    }

    // 1. Execute on the (untrusted) executor.
    let result = executor.execute(&spec.input)?;

    // 2. Independently verify against the trusted reference.
    let verdict = verify(&spec.input, &result, policy);

    // 3. Settle, only for paid jobs and only via the verdict.
    let settlement = match (&spec.mode, escrow) {
        (JobMode::Paid { .. }, Some(esc)) => Some(esc.settle(verdict)?),
        _ => None,
    };

    Ok(DispatchOutcome { result, verdict, settlement })
}

#[cfg(test)]
mod tests {
    use super::super::executor::{JobExecutor, LocalKernelExecutor};
    use super::super::job::{JobInput, JobMode, JobResult, JobSpec};
    use super::super::settlement::{Escrow, EscrowState, SettlementOutcome};
    use super::super::SwarmError;
    use super::*;

    fn dense_job(mode: JobMode) -> JobSpec {
        JobSpec::new(
            mode,
            JobInput::DenseLinearProduct {
                m: 2, k: 2, n: 2,
                a: vec![1.0, 2.0, 3.0, 4.0],
                b: vec![5.0, 6.0, 7.0, 8.0],
            },
        )
    }

    /// A dishonest executor that returns a wrong product — the adversary the
    /// verify-before-pay gate exists to stop.
    struct LyingExecutor;
    impl JobExecutor for LyingExecutor {
        fn execute(&self, input: &JobInput) -> Result<JobResult, SwarmError> {
            match input {
                JobInput::DenseLinearProduct { m, n, .. } => {
                    Ok(JobResult::DenseLinearProduct { c: vec![0.0; m * n] }) // all zeros — wrong
                }
                _ => Err(SwarmError::InvalidJob),
            }
        }
    }

    fn paid_mode() -> JobMode {
        JobMode::Paid { requester_did: 1, provider_did: 2, price_micro_units: 500 }
    }

    #[test]
    fn honest_paid_job_verifies_and_pays() {
        let spec = dense_job(paid_mode());
        let mut escrow = Escrow::offer(spec.id, 1, 2, 500, "$ilp.solar/pay", false);
        escrow.hold().unwrap();
        let out = run_job(&spec, &LocalKernelExecutor, VerifyPolicy::default(), Some(&mut escrow)).unwrap();
        assert!(out.verdict.is_verified());
        assert!(matches!(out.settlement, Some(SettlementOutcome::Pay(_))));
        assert_eq!(escrow.state, EscrowState::ReleasedToProvider);
    }

    #[test]
    fn lying_paid_job_is_rejected_and_refunded_never_paid() {
        let spec = dense_job(paid_mode());
        let mut escrow = Escrow::offer(spec.id, 1, 2, 500, "$ilp.solar/pay", false);
        escrow.hold().unwrap();
        let out = run_job(&spec, &LyingExecutor, VerifyPolicy::default(), Some(&mut escrow)).unwrap();
        assert!(!out.verdict.is_verified());
        assert!(
            matches!(out.settlement, Some(SettlementOutcome::Refund { .. })),
            "a lying provider must never be paid"
        );
        assert_eq!(escrow.state, EscrowState::RefundedToRequester);
    }

    #[test]
    fn personal_job_runs_with_no_settlement() {
        let spec = dense_job(JobMode::Personal);
        let out = run_job(&spec, &LocalKernelExecutor, VerifyPolicy::default(), None).unwrap();
        assert!(out.verdict.is_verified());
        assert!(out.settlement.is_none());
    }

    #[test]
    fn collaborative_job_runs_with_no_settlement() {
        let spec = dense_job(JobMode::Collaborative { peers: vec![7, 8] });
        let out = run_job(&spec, &LocalKernelExecutor, VerifyPolicy::default(), None).unwrap();
        assert!(out.verdict.is_verified());
        assert!(out.settlement.is_none());
    }
}
