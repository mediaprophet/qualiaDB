//! 6-dimensional Pareto frontier computation for experiment scoring.
//!
//! Each experiment is scored on six dimensions:
//! - Latency (lower is better)
//! - Throughput (higher is better)
//! - VRAM (lower is better)
//! - Quality (higher is better)
//! - Energy (lower is better)
//! - Cost (lower is better)
//!
//! Dominated results are pruned but retained for analysis.

use serde::{Deserialize, Serialize};

use super::experiment::ExperimentResult;

/// The six Pareto dimensions. "Lower is better" dimensions are negated
/// internally so that "higher is always better" in the dominance comparison.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParetoPoint {
    /// Latency in ms (lower is better → negated).
    pub latency_ms: f64,
    /// Throughput in tok/s (higher is better).
    pub throughput_tok_s: f64,
    /// VRAM in bytes (lower is better → negated).
    pub vram_bytes: f64,
    /// Quality score [0, 1] (higher is better).
    pub quality: f64,
    /// Energy in joules (lower is better → negated).
    pub energy_j: f64,
    /// Cost composite: VRAM × latency / throughput (lower is better → negated).
    pub cost: f64,
}

impl ParetoPoint {
    /// Extract a Pareto point from an experiment result.
    pub fn from_result(r: &ExperimentResult) -> Option<Self> {
        let bench = r.bench.as_ref()?;
        if r.error.is_some() {
            return None;
        }

        let latency_ms = bench.warm_total_ms.max(0.0);
        let throughput_tok_s = bench.decode_tok_s.max(0.0);
        let vram_bytes = r.vram_used as f64;
        let quality = r.quality.composite();
        let energy_j = r.thermal.energy_j.max(0.0);
        let cost = if throughput_tok_s > 0.0 {
            vram_bytes * latency_ms / throughput_tok_s
        } else {
            f64::INFINITY
        };

        Some(Self {
            latency_ms,
            throughput_tok_s,
            vram_bytes,
            quality,
            energy_j,
            cost,
        })
    }

    /// Convert to a "higher is better" vector for dominance comparison.
    fn to_higher_better(&self) -> [f64; 6] {
        [
            -self.latency_ms, // lower latency → higher negated
            self.throughput_tok_s,
            -self.vram_bytes, // lower VRAM → higher negated
            self.quality,
            -self.energy_j, // lower energy → higher negated
            -self.cost,     // lower cost → higher negated
        ]
    }

    /// Returns true if `self` dominates `other` (self is better or equal in all
    /// dimensions, and strictly better in at least one).
    pub fn dominates(&self, other: &ParetoPoint) -> bool {
        let a = self.to_higher_better();
        let b = other.to_higher_better();
        let mut any_strictly_better = false;
        for i in 0..6 {
            if a[i] < b[i] {
                return false;
            }
            if a[i] > b[i] {
                any_strictly_better = true;
            }
        }
        any_strictly_better
    }
}

/// A Pareto frontier: the set of non-dominated points from a collection of experiments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParetoFrontier {
    /// Indices into the original results vector for non-dominated points.
    pub non_dominated: Vec<usize>,
    /// Indices of dominated points (retained for analysis).
    pub dominated: Vec<usize>,
}

impl ParetoFrontier {
    /// Compute the Pareto frontier from a slice of experiment results.
    /// Returns a frontier with indices into the original slice.
    pub fn compute(results: &[ExperimentResult]) -> Self {
        let points: Vec<Option<ParetoPoint>> =
            results.iter().map(ParetoPoint::from_result).collect();

        let mut non_dominated = Vec::new();
        let mut dominated = Vec::new();

        for (i, pi) in points.iter().enumerate() {
            let Some(p_i) = pi else {
                // Failed experiments are neither dominated nor non-dominated.
                continue;
            };
            let mut is_dominated = false;
            for (j, pj) in points.iter().enumerate() {
                if i == j {
                    continue;
                }
                if let Some(p_j) = pj {
                    if p_j.dominates(p_i) {
                        is_dominated = true;
                        break;
                    }
                }
            }
            if is_dominated {
                dominated.push(i);
            } else {
                non_dominated.push(i);
            }
        }

        Self {
            non_dominated,
            dominated,
        }
    }

    /// Number of non-dominated points.
    pub fn frontier_size(&self) -> usize {
        self.non_dominated.len()
    }

    /// Get the non-dominated experiment results.
    pub fn frontier_results<'a>(
        &self,
        results: &'a [ExperimentResult],
    ) -> Vec<&'a ExperimentResult> {
        self.non_dominated
            .iter()
            .filter_map(|&i| results.get(i))
            .collect()
    }

    /// Serialize the frontier to JSON for external dashboard export.
    pub fn to_json(&self, results: &[ExperimentResult]) -> String {
        let frontier: Vec<&ExperimentResult> = self
            .non_dominated
            .iter()
            .filter_map(|&i| results.get(i))
            .collect();
        serde_json::to_string_pretty(
            &frontier
                .iter()
                .map(|r| {
                    let p = ParetoPoint::from_result(r);
                    serde_json::json!({
                        "config_hash": r.config_hash,
                        "hypothesis_id": r.hypothesis_id,
                        "pareto": p,
                        "quality": r.quality,
                        "decode_tok_s": r.bench.as_ref().map(|b| b.decode_tok_s),
                        "warm_total_ms": r.bench.as_ref().map(|b| b.warm_total_ms),
                        "vram_used": r.vram_used,
                        "energy_j": r.thermal.energy_j,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

/// Score a result against an application profile to select the "best" point
/// from the Pareto frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationProfileWeight {
    /// Optimize for latency (interactive).
    Interactive,
    /// Optimize for throughput (live-fast).
    LiveFast,
    /// Optimize for quality (batch-overnight).
    BatchOvernight,
}

impl ApplicationProfileWeight {
    /// Weight vector for scoring: (latency, throughput, vram, quality, energy, cost).
    fn weights(&self) -> [f64; 6] {
        match self {
            Self::Interactive => [0.35, 0.20, 0.10, 0.15, 0.10, 0.10],
            Self::LiveFast => [0.15, 0.40, 0.10, 0.10, 0.10, 0.15],
            Self::BatchOvernight => [0.10, 0.10, 0.10, 0.50, 0.10, 0.10],
        }
    }

    /// Score a Pareto point: higher is better. Normalizes each dimension to [0, 1]
    /// relative to the frontier min/max, then applies the weight vector.
    pub fn score(&self, point: &ParetoPoint, frontier: &[ParetoPoint]) -> f64 {
        if frontier.is_empty() {
            return 0.0;
        }
        let w = self.weights();
        let hb = point.to_higher_better();

        // Normalize each dimension relative to frontier min/max.
        let mut score = 0.0;
        for dim in 0..6 {
            let vals: Vec<f64> = frontier.iter().map(|p| p.to_higher_better()[dim]).collect();
            let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let normalized = if (hi - lo).abs() < 1e-12 {
                0.5
            } else {
                ((hb[dim] - lo) / (hi - lo)).clamp(0.0, 1.0)
            };
            score += w[dim] * normalized;
        }
        score
    }

    /// Select the best point from the frontier for this profile.
    pub fn select_best<'a>(
        &self,
        results: &'a [ExperimentResult],
        frontier: &ParetoFrontier,
    ) -> Option<&'a ExperimentResult> {
        let frontier_points: Vec<ParetoPoint> = frontier
            .non_dominated
            .iter()
            .filter_map(|&i| results.get(i).and_then(ParetoPoint::from_result))
            .collect();

        let mut best_idx: Option<usize> = None;
        let mut best_score = f64::NEG_INFINITY;
        for (fi, &ri) in frontier.non_dominated.iter().enumerate() {
            if let Some(ref p) = frontier_points.get(fi) {
                let s = self.score(p, &frontier_points);
                if s > best_score {
                    best_score = s;
                    best_idx = Some(ri);
                }
            }
        }
        best_idx.and_then(|i| results.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::lab::experiment::{
        BenchResultSerde, ExperimentResult, PhaseSnapshotSerde, QualityScore, ThermalSnapshot,
    };

    fn make_result(
        warm_total_ms: f64,
        decode_tok_s: f64,
        vram: u64,
        quality: f64,
        energy: f64,
    ) -> ExperimentResult {
        ExperimentResult {
            config_hash: rand::random(),
            hypothesis_id: None,
            bench: Some(BenchResultSerde {
                warm_total_ms,
                decode_tok_s,
                ..Default::default()
            }),
            phase: PhaseSnapshotSerde::default(),
            quality: QualityScore {
                pass_rate: quality,
                total_checks: 1,
                passed: if quality >= 1.0 { 1 } else { 0 },
                repaired: false,
                text_len: 0,
            },
            thermal: ThermalSnapshot {
                energy_j: energy,
                ..Default::default()
            },
            vram_used: vram,
            config_cbor: vec![],
            timestamp_ms: 0,
            seed: 0,
            error: None,
        }
    }

    #[test]
    fn pareto_dominance_basic() {
        let a = ParetoPoint {
            latency_ms: 100.0,
            throughput_tok_s: 50.0,
            vram_bytes: 1e9,
            quality: 0.95,
            energy_j: 10.0,
            cost: 2e9,
        };
        let b = ParetoPoint {
            latency_ms: 200.0,
            throughput_tok_s: 30.0,
            vram_bytes: 2e9,
            quality: 0.80,
            energy_j: 20.0,
            cost: 1.3e10,
        };
        // a dominates b in all dimensions.
        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn pareto_frontier_computes() {
        let results = vec![
            make_result(100.0, 50.0, 1_000_000_000, 0.95, 10.0), // best latency + throughput
            make_result(200.0, 30.0, 2_000_000_000, 0.80, 20.0), // dominated by 0
            make_result(150.0, 40.0, 500_000_000, 0.99, 15.0),   // best quality + VRAM
        ];
        let frontier = ParetoFrontier::compute(&results);
        assert_eq!(frontier.frontier_size(), 2);
        assert!(frontier.non_dominated.contains(&0));
        assert!(frontier.non_dominated.contains(&2));
        assert!(frontier.dominated.contains(&1));
    }

    #[test]
    fn profile_selects_best() {
        let results = vec![
            make_result(100.0, 50.0, 1_000_000_000, 0.90, 10.0),
            make_result(200.0, 30.0, 500_000_000, 0.99, 20.0),
        ];
        let frontier = ParetoFrontier::compute(&results);
        assert_eq!(frontier.frontier_size(), 2);

        // Interactive should prefer the lower latency result.
        let best = ApplicationProfileWeight::Interactive.select_best(&results, &frontier);
        assert!(best.is_some());
        let best = best.unwrap();
        assert!(best.bench.as_ref().unwrap().warm_total_ms <= 150.0);

        // BatchOvernight should prefer the higher quality result.
        let best = ApplicationProfileWeight::BatchOvernight.select_best(&results, &frontier);
        assert!(best.is_some());
        let best = best.unwrap();
        assert!(best.quality.pass_rate >= 0.95);
    }

    #[test]
    fn frontier_json_exports() {
        let results = vec![make_result(100.0, 50.0, 1_000_000_000, 0.95, 10.0)];
        let frontier = ParetoFrontier::compute(&results);
        let json = frontier.to_json(&results);
        assert!(json.contains("config_hash"));
        assert!(json.contains("pareto"));
    }
}
