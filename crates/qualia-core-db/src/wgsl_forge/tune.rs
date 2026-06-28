use serde::{Deserialize, Serialize};

use super::{
    AdapterConstraints, ComparisonReport, ForgeError, KernelSpec, Schedule, ScheduleSpace,
    TimingSource, TimingSummary,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEvaluation {
    pub oracle: ComparisonReport,
    pub timing_source: TimingSource,
    pub samples_ns: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateResult {
    pub schedule: Schedule,
    pub oracle: ComparisonReport,
    pub timing: TimingSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateFailure {
    pub schedule: Schedule,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuningConfig {
    pub initial_samples: usize,
    pub finalist_samples: usize,
    pub finalist_count: usize,
    pub max_candidates: usize,
}

impl Default for TuningConfig {
    fn default() -> Self {
        Self {
            initial_samples: 3,
            finalist_samples: 11,
            finalist_count: 6,
            max_candidates: 48,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningResult {
    pub evaluated_candidates: usize,
    pub rejected_candidates: usize,
    pub failures: Vec<CandidateFailure>,
    pub winner: CandidateResult,
    pub finalists: Vec<CandidateResult>,
}

pub fn tune_with<F>(
    kernel: &KernelSpec,
    constraints: &AdapterConstraints,
    space: &ScheduleSpace,
    config: TuningConfig,
    mut evaluate: F,
) -> Result<TuningResult, ForgeError>
where
    F: FnMut(Schedule, usize) -> Result<CandidateEvaluation, ForgeError>,
{
    if config.initial_samples == 0
        || config.finalist_samples == 0
        || config.finalist_count == 0
        || config.max_candidates == 0
    {
        return Err(ForgeError::InvalidSchedule(
            "tuning sample and candidate budgets must be non-zero".to_string(),
        ));
    }

    let candidates = space
        .candidates(kernel, constraints)
        .into_iter()
        .take(config.max_candidates)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(ForgeError::InvalidSchedule(
            "schedule space contains no adapter-compatible candidates".to_string(),
        ));
    }
    let evaluated_candidates = candidates.len();

    let mut accepted = Vec::new();
    let mut failures = Vec::new();
    let mut rejected_candidates = 0usize;
    for schedule in candidates {
        match evaluate(schedule, config.initial_samples) {
            Ok(evaluation) if evaluation.oracle.passed() => {
                if let Some(timing) =
                    TimingSummary::from_samples(evaluation.timing_source, &evaluation.samples_ns)
                {
                    accepted.push(CandidateResult {
                        schedule,
                        oracle: evaluation.oracle,
                        timing,
                    });
                } else {
                    rejected_candidates += 1;
                    failures.push(CandidateFailure {
                        schedule,
                        reason: "candidate produced no timing samples".to_string(),
                    });
                }
            }
            Ok(evaluation) => {
                rejected_candidates += 1;
                failures.push(CandidateFailure {
                    schedule,
                    reason: format!(
                        "oracle rejected {} value(s), first mismatch {:?}",
                        evaluation.oracle.mismatch_count, evaluation.oracle.first_mismatch
                    ),
                });
            }
            Err(error) => {
                rejected_candidates += 1;
                failures.push(CandidateFailure {
                    schedule,
                    reason: error.to_string(),
                });
            }
        }
    }
    if accepted.is_empty() {
        return Err(ForgeError::OracleMismatch(
            "no schedule passed both oracle and timing gates".to_string(),
        ));
    }

    sort_results(&mut accepted);
    accepted.truncate(config.finalist_count.min(accepted.len()));

    let mut finalists = Vec::with_capacity(accepted.len());
    for initial in accepted {
        let final_evaluation = match evaluate(initial.schedule, config.finalist_samples) {
            Ok(evaluation) => evaluation,
            Err(error) => {
                rejected_candidates += 1;
                failures.push(CandidateFailure {
                    schedule: initial.schedule,
                    reason: format!("finalist evaluation failed: {error}"),
                });
                continue;
            }
        };
        if !final_evaluation.oracle.passed() {
            rejected_candidates += 1;
            failures.push(CandidateFailure {
                schedule: initial.schedule,
                reason: "finalist failed the CPU oracle".to_string(),
            });
            continue;
        }
        let Some(timing) = TimingSummary::from_samples(
            final_evaluation.timing_source,
            &final_evaluation.samples_ns,
        ) else {
            rejected_candidates += 1;
            failures.push(CandidateFailure {
                schedule: initial.schedule,
                reason: "finalist produced no timing samples".to_string(),
            });
            continue;
        };
        finalists.push(CandidateResult {
            schedule: initial.schedule,
            oracle: final_evaluation.oracle,
            timing,
        });
    }
    if finalists.is_empty() {
        return Err(ForgeError::OracleMismatch(
            "all finalist schedules failed certification".to_string(),
        ));
    }
    sort_results(&mut finalists);
    let winner = finalists[0].clone();
    Ok(TuningResult {
        evaluated_candidates,
        rejected_candidates,
        failures,
        winner,
        finalists,
    })
}

fn sort_results(results: &mut [CandidateResult]) {
    results.sort_by(|left, right| {
        left.timing
            .median_ns
            .cmp(&right.timing.median_ns)
            .then(left.timing.p95_ns.cmp(&right.timing.p95_ns))
            .then(left.schedule.sort_key().cmp(&right.schedule.sort_key()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::{BuiltinKernel, OracleTolerance};

    fn passing_report() -> ComparisonReport {
        ComparisonReport {
            compared: 16,
            mismatch_count: 0,
            first_mismatch: None,
            max_absolute_error: OracleTolerance::default().absolute / 2.0,
            max_relative_error: 0.0,
        }
    }

    #[test]
    fn tuner_is_deterministic_and_correctness_gated() {
        let kernel = BuiltinKernel::AffineF32.spec();
        let constraints = AdapterConstraints::portable();
        let space = ScheduleSpace {
            workgroup_sizes: vec![32, 64],
            items_per_invocation: vec![1, 2],
            vector_widths: vec![1],
        };
        let run = || {
            tune_with(
                &kernel,
                &constraints,
                &space,
                TuningConfig {
                    initial_samples: 2,
                    finalist_samples: 3,
                    finalist_count: 2,
                    max_candidates: 4,
                },
                |schedule, samples| {
                    let base = 10_000 / schedule.elements_per_workgroup() as u64;
                    Ok(CandidateEvaluation {
                        oracle: passing_report(),
                        timing_source: TimingSource::Synthetic,
                        samples_ns: (0..samples).map(|index| base + index as u64).collect(),
                    })
                },
            )
            .unwrap()
        };
        assert_eq!(run(), run());
        assert_eq!(run().winner.schedule.workgroup_size, 64);
        assert_eq!(run().winner.schedule.items_per_invocation, 2);
    }
}
