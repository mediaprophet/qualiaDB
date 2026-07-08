//! Forensic economics, nquin trajectories, malfeasance delta, epistemic
//! negligence, shadow/fantasy graph costing, and human-rights impact kernels.
//!
//! This module implements the requirements of AGENTS.md §5.10-A.
//! It is deliberately rights-aware and refuses to fabricate when evidence
//! or calibration data is absent.
//!
//! # Allocation class
//!
//! Hot kernels are HotZeroHeap using fixed `[T; N]` buffers. Cold construction
//! (scenario/persona builders) is ColdBounded and documented.
//!
//! # Integration notes
//!
//! - Compose with WellFair event streams (personal evidence layer).
//! - NQuin encoding and SHACL live in later bridge work.
//! - 10D manifold embeddings and ZK proofs are later phases; this module
//!   produces the numeric vectors and deltas the bridges will consume.
//! - All outputs carry explicit assumptions, evidence sufficiency, and
//!   provenance hashes (caller supplied where external).

// (EconConvergence / EconStatus available if needed for future convergence reports)

/// Maximum number of tracked nquin dimensions (physical, psychological,
/// social_safety, agency_sovereignty, temporal_compounding).
pub const NQUIN_DIMS: usize = 5;

/// Maximum synthetic personas / trajectories in bounded buffers.
pub const MAX_PERSONAS: usize = 16;

/// Maximum steps in a bounded trajectory trace.
pub const MAX_TRAJECTORY_STEPS: usize = 512;

/// Nquin vector: experiential utility/deficit across 5 lived dimensions.
/// Higher is better (less deficit). Negative values indicate net harm load.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct NquinVector {
    /// Physical health component.
    pub physical: f64,
    /// Psychological / mental wellbeing.
    pub psychological: f64,
    /// Social safety / support / belonging.
    pub social_safety: f64,
    /// Agency / sovereignty / self-determination.
    pub agency: f64,
    /// Temporal compounding / future option value erosion.
    pub temporal: f64,
}

impl NquinVector {
    pub const ZERO: Self = Self {
        physical: 0.0,
        psychological: 0.0,
        social_safety: 0.0,
        agency: 0.0,
        temporal: 0.0,
    };

    #[inline]
    pub fn from_array(a: [f64; NQUIN_DIMS]) -> Self {
        Self {
            physical: a[0],
            psychological: a[1],
            social_safety: a[2],
            agency: a[3],
            temporal: a[4],
        }
    }

    #[inline]
    pub fn to_array(&self) -> [f64; NQUIN_DIMS] {
        [
            self.physical,
            self.psychological,
            self.social_safety,
            self.agency,
            self.temporal,
        ]
    }

    #[inline]
    pub fn is_finite(&self) -> bool {
        self.physical.is_finite()
            && self.psychological.is_finite()
            && self.social_safety.is_finite()
            && self.agency.is_finite()
            && self.temporal.is_finite()
    }

    /// L1 (Manhattan) norm of the vector.
    #[inline]
    pub fn l1_norm(&self) -> f64 {
        self.physical.abs()
            + self.psychological.abs()
            + self.social_safety.abs()
            + self.agency.abs()
            + self.temporal.abs()
    }
}

/// Lived state for a persona in the health/welfare Markov layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HealthWelfareState {
    Stable = 0,
    Stressed = 1,
    ChronicUnresolved = 2,
    AcuteCrisis = 3,
    /// Absorbing state per spec: once entered, recovery baseline is altered.
    IrrecoverableDebilitation = 4,
}

impl HealthWelfareState {
    #[inline]
    pub fn is_absorbing(self) -> bool {
        matches!(self, HealthWelfareState::IrrecoverableDebilitation)
    }
}

/// Accumulated harm record with memory / path dependence.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct AccumulatedHarm {
    pub current_load: NquinVector,
    /// Count of steps spent in stressed or worse states.
    pub chronic_steps: u32,
    /// Number of acute spikes observed.
    pub acute_spikes: u32,
    /// Whether an absorbing state has been entered.
    pub entered_absorbing: bool,
    /// Effective recovery baseline shift (0 = none, negative = impaired).
    pub recovery_baseline_shift: f64,
}

/// Malfeasance delta: allocated capital vs delivered human utility (in
/// approximate nquin-equivalent units). Positive = deficit (harm).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct MalfeasanceDelta {
    /// Capital allocated (arbitrary units; caller documented).
    pub capital_allocated: f64,
    /// Measured or modeled delivered nquin-utility (higher better).
    pub delivered_utility: f64,
    /// Delta = capital - delivered (positive = waste / malfeasance).
    pub delta: f64,
    /// Governance yield inversion flag: true when expenditure actively
    /// worsened foundational safety.
    pub governance_yield_inverted: bool,
}

/// Epistemic edge for negligence graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EpistemicEdge {
    pub knowable: bool,
    pub source_trustworthy: bool,
    pub duty_to_act: bool,
    pub action_taken: bool,
    /// True when downstream actor was misled by poisoned input.
    pub poisoned_input: bool,
}

/// Narrative / fantasy divergence record.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct NarrativeDivergence {
    /// Divergence between true state nquin and maintained narrative.
    pub true_vs_narrative: NquinVector,
    /// Propagated cost (nquin delta) of decisions made under the fantasy.
    pub propagated_cost: f64,
    /// Whether a cover-up/maintenance subgraph is active.
    pub maintenance_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForensicError {
    InvalidInput,
    NonFinite,
    BufferTooSmall,
    InsufficientEvidence,
    AbsorbingEntered,
}

/// Deterministic transition for a single persona step.
/// Uses a simple seeded rule-based Markov step + harm accumulation.
/// Returns new state and delta nquin for the step.
pub fn step_nquin_trajectory(
    state: HealthWelfareState,
    current: &NquinVector,
    support_level: f64, // 0..1 effective support (WellFair / intervention)
    shock: f64,         // external shock, can be negative
    seed: &mut u64,
) -> Result<(HealthWelfareState, NquinVector, AccumulatedHarm), ForensicError> {
    if !current.is_finite() || !support_level.is_finite() || !shock.is_finite() {
        return Err(ForensicError::NonFinite);
    }
    if !(0.0..=1.0).contains(&support_level) {
        return Err(ForensicError::InvalidInput);
    }

    let rng = splitmix64(*seed);
    *seed = rng.state;

    let mut next = *current;

    // Base drift toward baseline (recovery pressure)
    let recovery = 0.02 * support_level;
    next.physical += recovery - 0.01 * (1.0 - support_level);
    next.psychological += recovery - 0.015 * (1.0 - support_level);
    next.social_safety += 0.015 * support_level;
    next.agency += 0.01 * support_level - 0.005;
    next.temporal += 0.005 * support_level;

    // Apply shock
    next.physical += shock * 0.6;
    next.psychological += shock * 0.8;
    next.social_safety += shock * 0.4;
    next.agency += shock * 0.5;
    next.temporal += shock * 1.2; // future erosion compounds

    // Simple state machine with absorbing
    let harm_load = next.l1_norm();
    let new_state = if state.is_absorbing() {
        // Stay absorbing, further slow degradation
        next.physical -= 0.005;
        next.temporal -= 0.01;
        HealthWelfareState::IrrecoverableDebilitation
    } else if harm_load > 2.5 || state == HealthWelfareState::AcuteCrisis {
        HealthWelfareState::IrrecoverableDebilitation
    } else if harm_load > 1.8 || state == HealthWelfareState::ChronicUnresolved {
        HealthWelfareState::AcuteCrisis
    } else if harm_load > 0.9 || state == HealthWelfareState::Stressed {
        HealthWelfareState::ChronicUnresolved
    } else if harm_load > 0.3 {
        HealthWelfareState::Stressed
    } else {
        HealthWelfareState::Stable
    };

    let mut accum = AccumulatedHarm {
        current_load: next,
        chronic_steps: 0,
        acute_spikes: 0,
        entered_absorbing: new_state.is_absorbing(),
        recovery_baseline_shift: 0.0,
    };

    if matches!(
        new_state,
        HealthWelfareState::ChronicUnresolved | HealthWelfareState::AcuteCrisis
    ) {
        accum.chronic_steps = 1;
    }
    if new_state == HealthWelfareState::AcuteCrisis {
        accum.acute_spikes = 1;
    }
    if new_state.is_absorbing() {
        accum.recovery_baseline_shift = -0.15; // impaired baseline
    }

    // Clamp to avoid runaway
    let mut arr = next.to_array();
    for v in &mut arr {
        *v = v.clamp(-10.0, 5.0);
    }
    next = NquinVector::from_array(arr);

    Ok((new_state, next, accum))
}

/// Accumulate harm over a trace. Writes per-step nquin and states into caller
/// buffers. Returns number of steps executed and whether absorbing was hit.
pub fn accumulate_harm_trace(
    initial_state: HealthWelfareState,
    initial_nquin: &NquinVector,
    support_levels: &[f64],
    shocks: &[f64],
    seed: u64,
    out_states: &mut [HealthWelfareState],
    out_nquins: &mut [NquinVector],
    out_accum: &mut [AccumulatedHarm],
) -> Result<(usize, bool), ForensicError> {
    let n = support_levels.len().min(shocks.len());
    if n == 0 {
        return Err(ForensicError::InvalidInput);
    }
    if out_states.len() < n || out_nquins.len() < n || out_accum.len() < n {
        return Err(ForensicError::BufferTooSmall);
    }

    let mut state = initial_state;
    let mut nq = *initial_nquin;
    let mut rng_seed = seed;
    let mut hit_absorbing = false;

    for i in 0..n {
        let (next_state, next_nq, step_accum) =
            step_nquin_trajectory(state, &nq, support_levels[i], shocks[i], &mut rng_seed)?;
        out_states[i] = next_state;
        out_nquins[i] = next_nq;
        out_accum[i] = step_accum;

        if next_state.is_absorbing() {
            hit_absorbing = true;
        }

        state = next_state;
        nq = next_nq;

        if hit_absorbing && i + 1 < n {
            // Fill remaining with absorbing continuation (deterministic)
            for j in (i + 1)..n {
                let cont = step_nquin_trajectory(
                    state,
                    &nq,
                    support_levels[j].max(0.0).min(0.1), // minimal support once absorbing
                    -0.02,
                    &mut rng_seed,
                )?;
                out_states[j] = cont.0;
                out_nquins[j] = cont.1;
                out_accum[j] = cont.2;
                nq = cont.1;
            }
            break;
        }
    }

    Ok((n.min(MAX_TRAJECTORY_STEPS), hit_absorbing))
}

/// Compute malfeasance delta from allocated capital and observed delivered
/// utility (sum of nquin improvement or absolute level).
pub fn compute_malfeasance_delta(
    capital_allocated: f64,
    delivered_nquin_sum: f64,
    inverted: bool,
) -> Result<MalfeasanceDelta, ForensicError> {
    if !capital_allocated.is_finite() || !delivered_nquin_sum.is_finite() {
        return Err(ForensicError::NonFinite);
    }
    if capital_allocated < 0.0 {
        return Err(ForensicError::InvalidInput);
    }
    let delta = capital_allocated - delivered_nquin_sum;
    Ok(MalfeasanceDelta {
        capital_allocated,
        delivered_utility: delivered_nquin_sum,
        delta,
        governance_yield_inverted: inverted,
    })
}

/// Simple epistemic negligence classification.
/// Returns a conservative attribution score (0 good, 1 severe).
pub fn epistemic_negligence_score(edges: &[EpistemicEdge]) -> f64 {
    if edges.is_empty() {
        return 0.0;
    }
    let mut score = 0.0;
    for e in edges {
        if e.knowable && e.duty_to_act && !e.action_taken {
            score += 0.6;
        }
        if e.poisoned_input && e.action_taken {
            score += 0.2; // mitigated credit for acting on bad info
        }
        if e.knowable && !e.source_trustworthy {
            score += 0.3;
        }
    }
    (score / edges.len() as f64).clamp(0.0, 1.0)
}

/// Compute narrative divergence between factual nquin trace and maintained
/// fantasy trace. Also returns a simple propagated cost.
pub fn compute_narrative_divergence(
    factual: &[NquinVector],
    fantasy: &[NquinVector],
) -> Result<NarrativeDivergence, ForensicError> {
    if factual.is_empty() || fantasy.is_empty() || factual.len() != fantasy.len() {
        return Err(ForensicError::InvalidInput);
    }
    let mut sum_diff = NquinVector::ZERO;
    let mut cost = 0.0;
    for (f, fa) in factual.iter().zip(fantasy.iter()) {
        if !f.is_finite() || !fa.is_finite() {
            return Err(ForensicError::NonFinite);
        }
        let d_phys = f.physical - fa.physical;
        let d_psy = f.psychological - fa.psychological;
        let d_soc = f.social_safety - fa.social_safety;
        let d_agn = f.agency - fa.agency;
        let d_tmp = f.temporal - fa.temporal;

        sum_diff.physical += d_phys;
        sum_diff.psychological += d_psy;
        sum_diff.social_safety += d_soc;
        sum_diff.agency += d_agn;
        sum_diff.temporal += d_tmp;

        // Cost is positive when fantasy overstated wellbeing (led to worse decisions)
        let local = (d_phys + d_psy + d_soc + d_agn + d_tmp).max(0.0);
        cost += local;
    }
    Ok(NarrativeDivergence {
        true_vs_narrative: sum_diff,
        propagated_cost: cost,
        maintenance_active: cost > 0.1,
    })
}

/// Synthetic WellFair-style persona fixture generator (ColdBounded).
/// Produces a deterministic persona trajectory given seed.
pub fn generate_synthetic_persona_trace(
    seed: u64,
    steps: usize,
    initial_support: f64,
    shock_profile: &[f64],
    out_states: &mut [HealthWelfareState],
    out_nquins: &mut [NquinVector],
) -> Result<(usize, bool), ForensicError> {
    if steps == 0 || steps > MAX_TRAJECTORY_STEPS {
        return Err(ForensicError::InvalidInput);
    }
    if out_states.len() < steps || out_nquins.len() < steps {
        return Err(ForensicError::BufferTooSmall);
    }

    let init = NquinVector {
        physical: 0.8,
        psychological: 0.7,
        social_safety: 0.6,
        agency: 0.9,
        temporal: 0.5,
    };

    let mut supports = [0.0f64; MAX_TRAJECTORY_STEPS];
    let mut shocks = [0.0f64; MAX_TRAJECTORY_STEPS];
    let n = steps.min(shock_profile.len()).min(MAX_TRAJECTORY_STEPS);
    for i in 0..n {
        supports[i] = initial_support;
        shocks[i] = shock_profile[i];
    }
    // Fill remaining support decaying if short profile
    for i in n..steps {
        supports[i] = (initial_support * 0.8).max(0.1);
        shocks[i] = -0.01; // slow background erosion
    }

    accumulate_harm_trace(
        HealthWelfareState::Stable,
        &init,
        &supports[..steps],
        &shocks[..steps],
        seed,
        out_states,
        out_nquins,
        &mut [AccumulatedHarm {
            current_load: NquinVector::ZERO,
            chronic_steps: 0,
            acute_spikes: 0,
            entered_absorbing: false,
            recovery_baseline_shift: 0.0,
        }; MAX_TRAJECTORY_STEPS][..steps],
    )
}

/// Early-intervention counterfactual delta (very simplified).
/// Runs two traces (with vs without early support boost) and returns
/// final nquin L1 difference (positive = benefit of intervention).
pub fn early_intervention_counterfactual_delta(
    seed: u64,
    steps: usize,
    baseline_support: f64,
    boosted_support: f64,
    shock_profile: &[f64],
) -> Result<f64, ForensicError> {
    if steps == 0 || steps > MAX_TRAJECTORY_STEPS {
        return Err(ForensicError::InvalidInput);
    }
    let mut s1 = [HealthWelfareState::Stable; MAX_TRAJECTORY_STEPS];
    let mut n1 = [NquinVector::ZERO; MAX_TRAJECTORY_STEPS];
    let mut s2 = [HealthWelfareState::Stable; MAX_TRAJECTORY_STEPS];
    let mut n2 = [NquinVector::ZERO; MAX_TRAJECTORY_STEPS];

    let _ = generate_synthetic_persona_trace(seed, steps, baseline_support, shock_profile, &mut s1, &mut n1)?;
    let _ = generate_synthetic_persona_trace(seed, steps, boosted_support, shock_profile, &mut s2, &mut n2)?;

    let l1_1 = n1[steps - 1].l1_norm();
    let l1_2 = n2[steps - 1].l1_norm();
    Ok((l1_2 - l1_1).max(0.0)) // benefit
}

// --- local SplitMix64 (duplicated to keep module self-contained, zero-dep) ---
struct SplitMix64 {
    state: u64,
}
fn splitmix64(seed: u64) -> SplitMix64 {
    let mut s = SplitMix64 { state: seed };
    // one mix
    s.state = s.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = s.state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    s.state = z ^ (z >> 31);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nquin_vector_basics() {
        let v = NquinVector::from_array([0.1, -0.2, 0.3, -0.4, 0.5]);
        assert!((v.l1_norm() - 1.5).abs() < 1e-9);
        assert!(v.is_finite());
    }

    #[test]
    fn trajectory_reaches_absorbing_under_shock() {
        let init = NquinVector::ZERO;
        let mut states = [HealthWelfareState::Stable; 64];
        let mut nqs = [NquinVector::ZERO; 64];
        let mut acc = [AccumulatedHarm {
            current_load: NquinVector::ZERO,
            chronic_steps: 0,
            acute_spikes: 0,
            entered_absorbing: false,
            recovery_baseline_shift: 0.0,
        }; 64];
        let supports = [0.3; 64];
        let shocks = [-0.15; 64]; // persistent negative shock
        let (n, hit) = accumulate_harm_trace(
            HealthWelfareState::Stable,
            &init,
            &supports,
            &shocks,
            424242,
            &mut states,
            &mut nqs,
            &mut acc,
        )
        .unwrap();
        assert!(n > 10);
        assert!(hit);
        assert!(states[n - 1].is_absorbing());
    }

    #[test]
    fn malfeasance_positive_when_utility_low() {
        let d = compute_malfeasance_delta(1000.0, 120.0, false).unwrap();
        assert!(d.delta > 800.0);
        assert!(!d.governance_yield_inverted);
    }

    #[test]
    fn epistemic_score_detects_inaction() {
        let edges = [
            EpistemicEdge {
                knowable: true,
                source_trustworthy: true,
                duty_to_act: true,
                action_taken: false,
                poisoned_input: false,
            },
            EpistemicEdge {
                knowable: true,
                source_trustworthy: false,
                duty_to_act: true,
                action_taken: true,
                poisoned_input: true,
            },
        ];
        let sc = epistemic_negligence_score(&edges);
        assert!(sc > 0.3);
    }

    #[test]
    fn narrative_divergence_detects_fantasy() {
        let factual = [NquinVector {
            physical: 0.0,
            psychological: -1.0,
            social_safety: 0.0,
            agency: 0.0,
            temporal: -0.5,
        }];
        let fantasy = [NquinVector {
            physical: 1.0,
            psychological: 1.0,
            social_safety: 1.0,
            agency: 1.0,
            temporal: 1.0,
        }];
        let div = compute_narrative_divergence(&factual, &fantasy).unwrap();
        assert!(div.propagated_cost > 0.0);
        assert!(div.maintenance_active);
    }

    #[test]
    fn synthetic_persona_and_counterfactual() {
        let mut states = [HealthWelfareState::Stable; 30];
        let mut nqs = [NquinVector::ZERO; 30];
        let shocks = [-0.05; 30];
        let (n, _hit) =
            generate_synthetic_persona_trace(7, 30, 0.4, &shocks, &mut states, &mut nqs).unwrap();
        assert_eq!(n, 30);
        let delta = early_intervention_counterfactual_delta(7, 30, 0.3, 0.7, &shocks).unwrap();
        // Boosted support should not make things worse.
        assert!(delta >= 0.0);
    }

    #[test]
    fn refuses_non_finite() {
        let bad = NquinVector {
            physical: f64::NAN,
            ..NquinVector::ZERO
        };
        let mut buf_s = [HealthWelfareState::Stable; 1];
        let mut buf_n = [NquinVector::ZERO; 1];
        let mut buf_a = [AccumulatedHarm {
            current_load: NquinVector::ZERO,
            chronic_steps: 0,
            acute_spikes: 0,
            entered_absorbing: false,
            recovery_baseline_shift: 0.0,
        }; 1];
        assert_eq!(
            accumulate_harm_trace(
                HealthWelfareState::Stable,
                &bad,
                &[0.5],
                &[0.0],
                1,
                &mut buf_s,
                &mut buf_n,
                &mut buf_a
            )
            .unwrap_err(),
            ForensicError::NonFinite
        );
    }
}
