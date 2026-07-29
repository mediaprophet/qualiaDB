//! Transparent sleep analytics for Phase 2 (SLP-01..10).
//!
//! Non-diagnostic: cumulative sleep debt and weekly heatmap from duration summaries only.

use serde::{Deserialize, Serialize};

/// Default adult sleep target used for debt (8 hours). UI may override.
pub const DEFAULT_TARGET_SLEEP_MIN: f64 = 480.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SleepNightSample {
    pub night_unix: u32,
    pub duration_min: f64,
    pub efficiency: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SleepDebtReport {
    pub target_sleep_min: f64,
    pub nights_analyzed: u32,
    pub cumulative_debt_min: f64,
    pub avg_duration_min: f64,
    pub avg_efficiency: Option<f64>,
    /// True when avg efficiency < 70% and avg hours < 6 (mirrors sleep_debt.n3 pattern flag).
    pub chronic_sleep_debt_flag: bool,
    pub formula_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SleepHeatmapCell {
    pub day_label: String,
    pub night_unix: u32,
    pub duration_min: f64,
    /// 0.0 = well below target, 1.0 = at/above target.
    pub fill_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SleepHeatmapReport {
    pub target_sleep_min: f64,
    pub cells: Vec<SleepHeatmapCell>,
}

/// Compute cumulative sleep debt: sum of max(0, target - actual) over samples.
pub fn compute_sleep_debt(samples: &[SleepNightSample], target_min: f64) -> SleepDebtReport {
    let n = samples.len();
    if n == 0 {
        return SleepDebtReport {
            target_sleep_min: target_min,
            nights_analyzed: 0,
            cumulative_debt_min: 0.0,
            avg_duration_min: 0.0,
            avg_efficiency: None,
            chronic_sleep_debt_flag: false,
            formula_note:
                "No sleep nights in journal. Debt = Σ max(0, target − duration) per night.".into(),
        };
    }

    let mut debt = 0.0;
    let mut dur_sum = 0.0;
    let mut eff_sum = 0.0;
    let mut eff_count = 0u32;
    for s in samples {
        debt += (target_min - s.duration_min).max(0.0);
        dur_sum += s.duration_min;
        if let Some(e) = s.efficiency {
            eff_sum += e;
            eff_count += 1;
        }
    }

    let avg_dur = dur_sum / n as f64;
    let avg_eff = if eff_count > 0 {
        Some(eff_sum / eff_count as f64)
    } else {
        None
    };
    let chronic = avg_eff.map(|e| e < 70.0).unwrap_or(false) && avg_dur < 360.0;

    SleepDebtReport {
        target_sleep_min: target_min,
        nights_analyzed: n as u32,
        cumulative_debt_min: debt,
        avg_duration_min: avg_dur,
        avg_efficiency: avg_eff,
        chronic_sleep_debt_flag: chronic,
        formula_note: format!(
            "Debt = Σ max(0, {:.0}min − duration) over {n} night(s). \
             Chronic flag (non-diagnostic): avg efficiency < 70% AND avg duration < 6h.",
            target_min
        ),
    }
}

/// Build a 7-cell heatmap from the most recent samples (newest last).
pub fn compute_weekly_heatmap(samples: &[SleepNightSample], target_min: f64) -> SleepHeatmapReport {
    let take = samples.len().min(7);
    let slice = if take > 0 {
        &samples[samples.len() - take..]
    } else {
        &[]
    };

    let cells: Vec<SleepHeatmapCell> = slice
        .iter()
        .map(|s| {
            let fill = (s.duration_min / target_min).clamp(0.0, 1.0);
            SleepHeatmapCell {
                day_label: format!("{}", s.night_unix),
                night_unix: s.night_unix,
                duration_min: s.duration_min,
                fill_ratio: fill,
            }
        })
        .collect();

    SleepHeatmapReport {
        target_sleep_min: target_min,
        cells,
    }
}

/// Parse journal sleep summary JSON (`duration_min`, optional `efficiency`).
pub fn parse_sleep_summary_json(summary: &str) -> Option<(f64, Option<f64>)> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    let dur = v.get("duration_min")?.as_f64()?;
    let eff = v.get("efficiency").and_then(|x| x.as_f64());
    Some((dur, eff))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debt_accumulates_short_nights() {
        let samples = vec![
            SleepNightSample {
                night_unix: 1,
                duration_min: 360.0,
                efficiency: Some(75.0),
            },
            SleepNightSample {
                night_unix: 2,
                duration_min: 420.0,
                efficiency: Some(80.0),
            },
        ];
        let report = compute_sleep_debt(&samples, 480.0);
        assert!((report.cumulative_debt_min - 180.0).abs() < 0.01);
        assert_eq!(report.nights_analyzed, 2);
    }

    #[test]
    fn chronic_flag_requires_low_efficiency_and_duration() {
        let samples = vec![SleepNightSample {
            night_unix: 1,
            duration_min: 300.0,
            efficiency: Some(65.0),
        }];
        let report = compute_sleep_debt(&samples, 480.0);
        assert!(report.chronic_sleep_debt_flag);
    }

    #[test]
    fn heatmap_caps_at_seven() {
        let samples: Vec<_> = (0..10)
            .map(|i| SleepNightSample {
                night_unix: i,
                duration_min: 400.0 + i as f64,
                efficiency: None,
            })
            .collect();
        let hm = compute_weekly_heatmap(&samples, 480.0);
        assert_eq!(hm.cells.len(), 7);
        assert_eq!(hm.cells[0].night_unix, 3);
    }
}
