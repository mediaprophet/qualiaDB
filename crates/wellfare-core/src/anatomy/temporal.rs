//! S2 — the **temporal-state engine**.
//!
//! Slice 1 answers "where does load converge *right now*". This layer answers "how does it *build up
//! and recover over time*, and which interventions bend which subsystem's curve". A [`Factor`] becomes
//! a time-stamped [`FactorEvent`] with a dose scaler and per-system [`Kinetics`] (onset → clearance).
//! [`EnvironmentModulator`]s (heat, season) scale matching contributions over a window. A [`Timeline`]
//! integrates events + interventions + environment; sampling it at a time reproduces a slice-1 factor
//! set (so [`accumulate`](super::accumulate) / [`systemic_implications`](super::systemic_implications)
//! run unchanged on any instant), and [`Timeline::system_trajectory`] traces one system's net burden.
//!
//! **The point of the layer** (the hot-week / beer / water example): different subsystems recover on
//! different clocks and respond to different interventions. Alcohol on the hepatic/digestive system
//! clears slowly on its own half-life — water and electrolytes do **not** speed it. Diuretic fluid loss
//! on the renal/urinary system *is* offset by a rehydration intervention, so it recovers far sooner.
//! The engine shows that divergence; it never asserts one.
//!
//! **Honesty:** all magnitudes stay integer milli (no float health arithmetic — the kinetic curves are
//! evaluated with integer math). Temporal projection is **coarse and illustrative**: [`RecoveryBand`]
//! is "hours / days / weeks", with explicit uncertainty. It is **never** an operational safety
//! threshold — no BAC number, no fitness-to-drive/operate claim.

use serde::{Deserialize, Serialize};

use super::factor::{Effect, Factor, FactorKind, FactorTarget};
use super::system_key;
use super::{accumulate::accumulate, accumulate::SystemBurden};

/// How a factor-event's contribution to a system rises to a peak, then clears.
///
/// - `onset_minutes`: time from the event to peak contribution (0 = immediate).
/// - `half_life_minutes`: clearance half-life *after* the peak. `0` means it does **not** clear while
///   present (a chronic condition, a standing environmental exposure) — it holds at peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kinetics {
    pub onset_minutes: u32,
    pub half_life_minutes: u32,
}

impl Kinetics {
    /// Immediate onset, never clears (chronic / standing).
    pub const CHRONIC: Kinetics = Kinetics { onset_minutes: 0, half_life_minutes: 0 };

    pub fn new(onset_minutes: u32, half_life_minutes: u32) -> Self {
        Self { onset_minutes, half_life_minutes }
    }

    /// Effective magnitude of a `base_milli` contribution `elapsed_minutes` after the event, given a
    /// dose scaler (percent; 100 = as authored). Integer math throughout; result clamped to 0..=1000.
    ///
    /// Rise is linear to the peak over `onset_minutes`; decay halves every `half_life_minutes` with a
    /// linear interpolation inside each half-life (coarse but monotone and honest).
    pub fn magnitude_at(self, base_milli: u32, dose_scale_pct: u32, elapsed_minutes: i64) -> u32 {
        if elapsed_minutes < 0 {
            return 0; // the event has not happened yet
        }
        let e = elapsed_minutes as u64;
        let peak = (base_milli as u64 * dose_scale_pct as u64 / 100).min(1000);
        if peak == 0 {
            return 0;
        }
        let onset = self.onset_minutes as u64;
        let shaped = if onset > 0 && e < onset {
            peak * e / onset // linear rise to peak
        } else if self.half_life_minutes == 0 {
            peak // chronic / standing — never clears
        } else {
            let since_peak = e - onset; // e >= onset (onset may be 0)
            let hl = self.half_life_minutes as u64;
            let n = (since_peak / hl).min(63); // full halvings (cap to avoid shift overflow → 0)
            let r = since_peak % hl;
            let hi = peak >> n;
            let lo = peak >> (n + 1).min(63);
            hi - (hi - lo) * r / hl // linear interpolation within the current half-life
        };
        shaped.min(1000) as u32
    }
}

/// A [`Factor`] applied at a point in time, with a dose scaler and per-system kinetics.
///
/// The factor's slice-1 `targets` supply the (system, effect, evidence, base weight); kinetics decide
/// how each target's contribution evolves. A target's kinetics come from `system_kinetics` if present,
/// else `default_kinetics` — this is what lets one event clear on different clocks per system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorEvent {
    pub factor: Factor,
    /// Event time on the timeline, in minutes (may be negative for "before now").
    pub at_minute: i64,
    /// Scales every target weight (percent; 100 = as authored). A "slab" of beer vs a single can.
    pub dose_scale_pct: u32,
    /// Fallback kinetics for targets without a per-system override.
    pub default_kinetics: Kinetics,
    /// Per-system kinetics overrides (`system_id` → kinetics).
    #[serde(default)]
    pub system_kinetics: Vec<(String, Kinetics)>,
}

impl FactorEvent {
    pub fn new(factor: Factor, at_minute: i64) -> Self {
        Self {
            factor,
            at_minute,
            dose_scale_pct: 100,
            default_kinetics: Kinetics::CHRONIC,
            system_kinetics: Vec::new(),
        }
    }

    /// Set the dose scaler (percent; 100 = as authored).
    pub fn with_dose_pct(mut self, dose_scale_pct: u32) -> Self {
        self.dose_scale_pct = dose_scale_pct;
        self
    }

    /// Set the fallback kinetics used for targets without a per-system override.
    pub fn with_default_kinetics(mut self, k: Kinetics) -> Self {
        self.default_kinetics = k;
        self
    }

    /// Override the kinetics for a specific system id (e.g. hepatic clears slower than renal).
    pub fn with_system_kinetics(mut self, system_id: impl Into<String>, k: Kinetics) -> Self {
        self.system_kinetics.push((system_id.into(), k));
        self
    }

    fn kinetics_for(&self, system_id: &str) -> Kinetics {
        self.system_kinetics
            .iter()
            .find(|(id, _)| system_key(id) == system_key(system_id))
            .map(|(_, k)| *k)
            .unwrap_or(self.default_kinetics)
    }
}

/// An environmental condition (heat, season, sustained activity) that scales matching contributions
/// over a time window. Entered or imported — never a live weather/cloud call.
///
/// `scale_pct` amplifies (>100) or dampens (<100) the magnitude of contributions whose system and
/// effect match `target_system` / `target_effect` (`None` = match any) while `now` is within
/// `[from_minute, to_minute]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentModulator {
    pub label: String,
    pub from_minute: i64,
    pub to_minute: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_effect: Option<Effect>,
    pub scale_pct: u32,
}

impl EnvironmentModulator {
    fn applies(&self, now: i64, system_id: &str, effect: Effect) -> bool {
        if now < self.from_minute || now > self.to_minute {
            return false;
        }
        if let Some(sys) = &self.target_system {
            if system_key(sys) != system_key(system_id) {
                return false;
            }
        }
        match self.target_effect {
            Some(e) if e != effect => false,
            _ => true,
        }
    }
}

/// A person's timeline of factor events + interventions + environment. Interventions are simply
/// `FactorEvent`s carrying `Effect::Supportive` targets — no separate type, so they net against
/// adverse load through the same slice-1 accumulation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Timeline {
    pub events: Vec<FactorEvent>,
    #[serde(default)]
    pub environment: Vec<EnvironmentModulator>,
}

/// One sampled point of a system's burden over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub minute: i64,
    pub net_milli: u32,
    pub adverse_milli: u32,
    pub supportive_milli: u32,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_event(mut self, e: FactorEvent) -> Self {
        self.events.push(e);
        self
    }

    pub fn with_environment(mut self, m: EnvironmentModulator) -> Self {
        self.environment.push(m);
        self
    }

    /// Reconstruct the slice-1 factor set as it stands at `minute`: each event contributes a [`Factor`]
    /// whose target weights are its kinetic magnitude at that instant, scaled by any active environment.
    /// Targets that have decayed (or not yet started) to zero are dropped; a fully-cleared event drops
    /// out entirely — so it stops being a "converging factor" once it's gone.
    pub fn snapshot_at(&self, minute: i64) -> Vec<Factor> {
        let mut out = Vec::new();
        for ev in &self.events {
            let elapsed = minute - ev.at_minute;
            let mut targets: Vec<FactorTarget> = Vec::new();
            for t in &ev.factor.targets {
                let base = ev.kinetics_for(&t.system_id).magnitude_at(
                    t.weight_milli,
                    ev.dose_scale_pct,
                    elapsed,
                );
                if base == 0 {
                    continue;
                }
                let scaled = self.apply_environment(minute, &t.system_id, t.effect, base);
                if scaled == 0 {
                    continue;
                }
                targets.push(FactorTarget {
                    system_id: t.system_id.clone(),
                    effect: t.effect,
                    evidence: t.evidence,
                    weight_milli: scaled,
                });
            }
            if targets.is_empty() {
                continue;
            }
            out.push(Factor {
                id: ev.factor.id.clone(),
                kind: ev.factor.kind.clone(),
                label: ev.factor.label.clone(),
                targets,
                source: ev.factor.source.clone(),
            });
        }
        out
    }

    /// Per-system burden at `minute` (slice-1 [`accumulate`] over the snapshot).
    pub fn burden_at(&self, minute: i64) -> Vec<SystemBurden> {
        accumulate(&self.snapshot_at(minute))
    }

    /// Trace one system's net/adverse/supportive burden across the given sample minutes.
    pub fn system_trajectory(&self, system_id: &str, samples: &[i64]) -> Vec<TrajectoryPoint> {
        samples
            .iter()
            .map(|&minute| {
                let burden = self
                    .burden_at(minute)
                    .into_iter()
                    .find(|b| system_key(&b.system_id) == system_key(system_id));
                match burden {
                    Some(b) => TrajectoryPoint {
                        minute,
                        net_milli: b.net_milli,
                        adverse_milli: b.adverse_milli,
                        supportive_milli: b.supportive_milli,
                    },
                    None => TrajectoryPoint {
                        minute,
                        net_milli: 0,
                        adverse_milli: 0,
                        supportive_milli: 0,
                    },
                }
            })
            .collect()
    }

    fn apply_environment(&self, minute: i64, system_id: &str, effect: Effect, base: u32) -> u32 {
        let mut v = base as u64;
        for m in &self.environment {
            if m.applies(minute, system_id, effect) {
                v = v * m.scale_pct as u64 / 100;
            }
        }
        v.min(1000) as u32
    }
}

/// A coarse, honest recovery horizon — **never** a precise time or a safety threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryBand {
    /// Returns toward baseline within ~half a day.
    Hours,
    /// Within a few days.
    Days,
    /// Within a few weeks.
    Weeks,
    /// Longer than weeks within the sampled window (or a standing/chronic load).
    Extended,
}

/// Classify a system trajectory's recovery horizon into a coarse band. Finds the peak net burden, then
/// the first later sample at or below a near-baseline threshold (10% of peak, floored at 20 milli), and
/// buckets the elapsed time. Returns `None` if there is no adverse load to recover from.
///
/// **Coarse and illustrative** — the band answers "hours vs days vs weeks", not a clock time, and never
/// implies a fitness-to-operate judgement. The trajectory must be sampled finely enough to catch the
/// crossing; a load still above threshold at the last sample is [`RecoveryBand::Extended`].
pub fn recovery_band(points: &[TrajectoryPoint]) -> Option<RecoveryBand> {
    let peak_idx = points.iter().enumerate().max_by_key(|(_, p)| p.net_milli).map(|(i, _)| i)?;
    let peak = points[peak_idx].net_milli;
    if peak == 0 {
        return None; // nothing adverse to recover from
    }
    let threshold = (peak / 10).max(20);
    let peak_minute = points[peak_idx].minute;
    let recovered_at = points[peak_idx..]
        .iter()
        .find(|p| p.net_milli <= threshold)
        .map(|p| p.minute - peak_minute);
    Some(match recovered_at {
        Some(mins) if mins <= 12 * 60 => RecoveryBand::Hours,
        Some(mins) if mins <= 3 * 24 * 60 => RecoveryBand::Days,
        Some(mins) if mins <= 21 * 24 * 60 => RecoveryBand::Weeks,
        _ => RecoveryBand::Extended,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::EvidenceTier;

    fn minutes_over(hours_end: i64, step_min: i64) -> Vec<i64> {
        (0..=(hours_end * 60 / step_min)).map(|i| i * step_min).collect()
    }

    #[test]
    fn kinetics_rise_peak_and_halve() {
        let k = Kinetics::new(60, 120); // 1h onset, 2h half-life
        assert_eq!(k.magnitude_at(400, 100, -10), 0); // before the event
        assert_eq!(k.magnitude_at(400, 100, 0), 0); // at the event, pre-onset
        assert_eq!(k.magnitude_at(400, 100, 30), 200); // half-way up the rise
        assert_eq!(k.magnitude_at(400, 100, 60), 400); // peak
        assert_eq!(k.magnitude_at(400, 100, 180), 200); // one half-life past peak
        assert_eq!(k.magnitude_at(400, 100, 300), 100); // two half-lives past peak
        // Decays toward zero over many half-lives.
        assert!(k.magnitude_at(400, 100, 60 + 120 * 10) < 5);
    }

    #[test]
    fn dose_scaling_amplifies_but_stays_bounded() {
        let k = Kinetics::new(0, 0); // immediate, chronic
        assert_eq!(k.magnitude_at(300, 100, 0), 300);
        assert_eq!(k.magnitude_at(300, 200, 0), 600); // double dose
        assert_eq!(k.magnitude_at(300, 1000, 0), 1000); // clamped to the model ceiling
    }

    #[test]
    fn chronic_kinetics_never_clears() {
        let k = Kinetics::CHRONIC;
        assert_eq!(k.magnitude_at(500, 100, 0), 500);
        assert_eq!(k.magnitude_at(500, 100, 100_000), 500);
    }

    #[test]
    fn snapshot_reproduces_a_slice1_factor_set_that_accumulates() {
        let beer = Factor::new("intake:beer", FactorKind::Food, "beer").targeting(
            "digestive",
            Effect::Adverse,
            EvidenceTier::Mechanistic,
            300,
        );
        let tl = Timeline::new().with_event(
            FactorEvent::new(beer, 0).with_default_kinetics(Kinetics::new(30, 240)),
        );
        // At peak the snapshot has one loaded factor; far in the future it has cleared away.
        let at_peak = tl.snapshot_at(30);
        assert_eq!(at_peak.len(), 1);
        assert!(at_peak[0].targets[0].weight_milli > 0);
        assert!(tl.snapshot_at(100_000).is_empty(), "fully-cleared event drops out of the snapshot");
    }

    #[test]
    fn environment_amplifies_matching_adverse_load() {
        let sweat = Factor::new("intake:none", FactorKind::Environmental, "fluid loss").targeting(
            "urinary",
            Effect::Adverse,
            EvidenceTier::Mechanistic,
            300,
        );
        let base = Timeline::new()
            .with_event(FactorEvent::new(sweat.clone(), 0).with_default_kinetics(Kinetics::CHRONIC));
        let hot = base.clone().with_environment(EnvironmentModulator {
            label: "heatwave".into(),
            from_minute: 0,
            to_minute: 10_000,
            target_system: Some("urinary".into()),
            target_effect: Some(Effect::Adverse),
            scale_pct: 150,
        });
        let cool_net = base.burden_at(60).into_iter().find(|b| b.system_id == "urinary").unwrap().net_milli;
        let hot_net = hot.burden_at(60).into_iter().find(|b| b.system_id == "urinary").unwrap().net_milli;
        assert_eq!(cool_net, 300);
        assert_eq!(hot_net, 450, "heat amplifies the renal/fluid load by 50%");
    }

    /// The load-bearing example: a hot week + a slab of beer + water later. The renal/urinary system
    /// (offset by rehydration) must recover on a *faster* clock than the hepatic/digestive system
    /// (alcohol clearance — time + liver only), which the water does not touch.
    #[test]
    fn hot_week_beer_and_water_recover_on_different_clocks() {
        // One intake event: hepatic load (slow, long half-life) + renal fluid load (medium).
        let beer = Factor::new("intake:slab-of-beer", FactorKind::Food, "a slab of beer")
            .targeting("digestive", Effect::Adverse, EvidenceTier::Mechanistic, 300) // hepatic/alcohol
            .targeting("urinary", Effect::Adverse, EvidenceTier::Mechanistic, 300); // diuresis → fluid loss
        let beer_event = FactorEvent::new(beer, 0)
            .with_dose_pct(300) // "a slab", not a can
            .with_system_kinetics("digestive", Kinetics::new(30, 300)) // alcohol clears slowly
            .with_system_kinetics("urinary", Kinetics::new(60, 180)); // fluid turns over faster

        // Rehydration intervention 2h in: supportive on the renal/urinary system only.
        let water = Factor::new("act:water-electrolytes", FactorKind::Lifestyle, "water + electrolytes")
            .targeting("urinary", Effect::Supportive, EvidenceTier::Mechanistic, 600);
        let water_event =
            FactorEvent::new(water, 120).with_default_kinetics(Kinetics::new(20, 240));

        // A week-long heatwave amplifies the renal fluid load (worse dehydration in the heat).
        let tl = Timeline::new().with_event(beer_event).with_event(water_event).with_environment(
            EnvironmentModulator {
                label: "summer heatwave".into(),
                from_minute: 0,
                to_minute: 7 * 24 * 60,
                target_system: Some("urinary".into()),
                target_effect: Some(Effect::Adverse),
                scale_pct: 140,
            },
        );

        let samples = minutes_over(48, 15); // 48h, every 15 min
        let renal = tl.system_trajectory("urinary", &samples);
        let hepatic = tl.system_trajectory("digestive", &samples);

        let renal_band = recovery_band(&renal).unwrap();
        let hepatic_band = recovery_band(&hepatic).unwrap();

        // Renal recovers within hours (water offsets it); hepatic takes longer (no offset).
        assert_eq!(renal_band, RecoveryBand::Hours, "rehydration bends the renal curve down fast");
        assert_ne!(hepatic_band, RecoveryBand::Hours, "alcohol clearance is not sped by water");

        // Concretely: by 12h the renal net is near baseline while the hepatic net is still loaded.
        let at_12h = |pts: &[TrajectoryPoint]| pts.iter().find(|p| p.minute == 12 * 60).unwrap().net_milli;
        let renal_12h = at_12h(&renal);
        let hepatic_12h = at_12h(&hepatic);
        assert!(renal_12h < 60, "renal recovered by 12h: {renal_12h}");
        assert!(hepatic_12h > renal_12h, "hepatic still more loaded than renal at 12h: {hepatic_12h} vs {renal_12h}");
    }

    #[test]
    fn recovery_band_none_when_no_adverse_load() {
        let pts = vec![
            TrajectoryPoint { minute: 0, net_milli: 0, adverse_milli: 0, supportive_milli: 0 },
            TrajectoryPoint { minute: 60, net_milli: 0, adverse_milli: 0, supportive_milli: 0 },
        ];
        assert!(recovery_band(&pts).is_none());
    }

    #[test]
    fn timeline_serde_round_trips() {
        let tl = Timeline::new().with_event(
            FactorEvent::new(
                Factor::new("f", FactorKind::Herb, "f").targeting(
                    "digestive",
                    Effect::Supportive,
                    EvidenceTier::TraditionalUse,
                    100,
                ),
                0,
            )
            .with_dose_pct(150)
            .with_system_kinetics("digestive", Kinetics::new(10, 60)),
        );
        let json = serde_json::to_string(&tl).unwrap();
        let back: Timeline = serde_json::from_str(&json).unwrap();
        assert_eq!(tl, back);
    }
}
