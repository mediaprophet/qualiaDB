//! Deterministic render-quality admission profiles shared by native and WASM callers.
//!
//! This module consumes a normalized capability snapshot instead of probing platform APIs.
//! Native adapters and browser/WebGPU probes can therefore produce the same decisions from the
//! same inputs. Unknown or partial probes choose a conservative profile and never infer support.

use super::acceptance_contract::{CapabilityAcceptance, CapabilityEvidence};

/// GPU features that may be independently enabled by a confirmed capability probe.
pub mod feature {
    pub const SHADOWS: u32 = 1 << 0;
    pub const AMBIENT_OCCLUSION: u32 = 1 << 1;
    pub const BLOOM: u32 = 1 << 2;
    pub const HDR: u32 = 1 << 3;
    pub const VOLUME_PROJECTION: u32 = 1 << 4;
}

/// Stable quality bands. They describe work budgets, not visual style; stylized and
/// photorealistic content share the same admission rules and may use different materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum QualityTier {
    Conservative = 0,
    Low = 1,
    Balanced = 2,
    High = 3,
    Ultra = 4,
}

/// A normalized probe result. `None` means the value was not measured or exposed by the runtime.
/// `confirmed_features` is deliberately explicit: absence means unsupported or not verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphicsCapabilitySnapshot {
    pub probe_complete: bool,
    pub tier_hint: Option<QualityTier>,
    pub max_texture_dimension_2d: Option<u32>,
    /// Available GPU allocation budget after the runtime's own safety margin, when reported.
    pub gpu_memory_budget_bytes: Option<u64>,
    pub confirmed_features: u32,
    /// Measured runtime evidence for cross-platform acceptance gates. The default is entirely
    /// unknown; callers must populate evidence instead of relying on a platform assumption.
    pub acceptance: CapabilityAcceptance,
}

impl Default for GraphicsCapabilitySnapshot {
    fn default() -> Self {
        Self {
            probe_complete: false,
            tier_hint: None,
            max_texture_dimension_2d: None,
            gpu_memory_budget_bytes: None,
            confirmed_features: 0,
            acceptance: CapabilityAcceptance::default(),
        }
    }
}

impl GraphicsCapabilitySnapshot {
    /// Gate for admitting the requested tier. Conservative is always a valid presentation
    /// baseline; every higher tier requires measured performance evidence.
    pub const fn quality_tier_gate(self) -> CapabilityEvidence {
        match self.tier_hint {
            None | Some(QualityTier::Conservative) => CapabilityEvidence::Confirmed,
            Some(_) if !self.probe_complete => CapabilityEvidence::Unknown,
            Some(_) => self.acceptance.performance_gate(),
        }
    }

    pub const fn temporal_gate(self) -> CapabilityEvidence {
        self.acceptance.temporal_gate()
    }

    pub const fn environment_probe_gate(self) -> CapabilityEvidence {
        self.acceptance.environment_probe_gate()
    }

    pub const fn browser_webgpu_gate(self) -> CapabilityEvidence {
        self.acceptance.browser_webgpu_gate()
    }

    pub const fn pixel_readback_gate(self) -> CapabilityEvidence {
        self.acceptance.pixel_readback_gate()
    }
}

/// Explicit per-frame and residency budgets selected from a capability snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderQualityProfile {
    pub tier: QualityTier,
    /// Render-target scale in basis points (10_000 = native resolution).
    pub render_scale_bps: u16,
    pub shadows_enabled: bool,
    pub shadow_map_dimension: u16,
    pub shadow_cascade_count: u8,
    pub ambient_occlusion_enabled: bool,
    /// AO target scale in basis points relative to the render target.
    pub ao_scale_bps: u16,
    pub bloom_enabled: bool,
    pub bloom_levels: u8,
    /// Maximum resident texture allocation. This is an asset/GPU residency budget and is
    /// independent of the semantic execution Sentinel budget.
    pub texture_residency_bytes: u64,
    pub hdr_enabled: bool,
    pub volume_projection_enabled: bool,
}

/// Select a deterministic profile. The shared VibeScript policy selects the tier from normalized
/// probe facts; partial/failed probes or script errors choose Conservative. Optional effects are
/// still admitted independently when their support was positively confirmed. This is a cold
/// profile-construction API, not a per-frame call.
pub fn select_quality_profile(snapshot: GraphicsCapabilitySnapshot) -> RenderQualityProfile {
    let tier = select_quality_tier_via_vibe(snapshot);
    let (scale, shadow_dim, cascades, ao_scale, bloom_levels, resident_share_bps, resident_cap) =
        match tier {
            QualityTier::Conservative => (5_000, 0, 0, 0, 0, 1_000, 64 * MIB),
            QualityTier::Low => (6_700, 1_024, 1, 5_000, 2, 1_500, 256 * MIB),
            QualityTier::Balanced => (8_500, 2_048, 2, 6_700, 3, 2_000, 1_024 * MIB),
            QualityTier::High => (10_000, 4_096, 4, 8_500, 5, 2_500, 2_048 * MIB),
            QualityTier::Ultra => (10_000, 8_192, 6, 10_000, 6, 3_500, 4_096 * MIB),
        };

    let dimension = snapshot.max_texture_dimension_2d.unwrap_or(0);
    let shadows_enabled = shadow_dim != 0
        && dimension >= shadow_dim as u32
        && has(snapshot.confirmed_features, feature::SHADOWS);
    let shadow_map_dimension = if shadows_enabled { shadow_dim } else { 0 };
    let ambient_occlusion_enabled = ao_scale != 0
        && dimension >= 256
        && has(snapshot.confirmed_features, feature::AMBIENT_OCCLUSION);
    let bloom_enabled =
        bloom_levels != 0 && dimension >= 256 && has(snapshot.confirmed_features, feature::BLOOM);
    let texture_residency_bytes = match snapshot.gpu_memory_budget_bytes {
        Some(available) => available
            .saturating_mul(resident_share_bps as u64)
            .checked_div(10_000)
            .unwrap_or(0)
            .min(resident_cap),
        None => 64 * MIB,
    };

    RenderQualityProfile {
        tier,
        render_scale_bps: scale,
        shadows_enabled,
        shadow_map_dimension,
        shadow_cascade_count: if shadows_enabled { cascades } else { 0 },
        ambient_occlusion_enabled,
        ao_scale_bps: if ambient_occlusion_enabled {
            ao_scale
        } else {
            0
        },
        bloom_enabled,
        bloom_levels: if bloom_enabled { bloom_levels } else { 0 },
        texture_residency_bytes,
        hdr_enabled: has(snapshot.confirmed_features, feature::HDR),
        volume_projection_enabled: has(snapshot.confirmed_features, feature::VOLUME_PROJECTION),
    }
}

fn select_quality_tier_via_vibe(snapshot: GraphicsCapabilitySnapshot) -> QualityTier {
    use vibe::{eval_function, load_program, Env, LocalHost, Value};

    let hint = match snapshot.tier_hint {
        Some(QualityTier::Conservative) | None => "conservative",
        Some(QualityTier::Low) => "low",
        Some(QualityTier::Balanced) => "balanced",
        Some(QualityTier::High) => "high",
        Some(QualityTier::Ultra) => "ultra",
    };
    let Ok(program) = load_program(include_str!("portal/graphics_backend.vibe")) else {
        return QualityTier::Conservative;
    };
    let mut host = LocalHost::default();
    let mut env = Env::default();
    let quality_probe_complete = snapshot.probe_complete
        && snapshot.quality_tier_gate() == CapabilityEvidence::Confirmed;
    let Ok(Value::String(result)) = eval_function(
        &program,
        "select_quality_tier",
        vec![
            Value::Bool(quality_probe_complete),
            Value::String(hint.to_owned()),
        ],
        &mut host,
        &mut env,
    ) else {
        return QualityTier::Conservative;
    };
    match result.as_str() {
        "low" => QualityTier::Low,
        "balanced" => QualityTier::Balanced,
        "high" => QualityTier::High,
        "ultra" => QualityTier::Ultra,
        _ => QualityTier::Conservative,
    }
}

const MIB: u64 = 1_048_576;

#[inline]
fn has(mask: u32, bit: u32) -> bool {
    mask & bit != 0
}

#[cfg(test)]
mod tests {
    use super::{
        feature, select_quality_profile, select_quality_tier_via_vibe, GraphicsCapabilitySnapshot,
        QualityTier,
    };
    use crate::render::acceptance_contract::CapabilityEvidence;

    fn capable(tier: QualityTier) -> GraphicsCapabilitySnapshot {
        GraphicsCapabilitySnapshot {
            probe_complete: true,
            tier_hint: Some(tier),
            max_texture_dimension_2d: Some(16_384),
            gpu_memory_budget_bytes: Some(8 * 1_048_576 * 1_024),
            confirmed_features: feature::SHADOWS
                | feature::AMBIENT_OCCLUSION
                | feature::BLOOM
                | feature::HDR
                | feature::VOLUME_PROJECTION,
            acceptance: crate::render::acceptance_contract::CapabilityAcceptance {
                performance: crate::render::acceptance_contract::PerformanceEvidence::measured(
                    3, 12_000, 16_667,
                ),
                ..Default::default()
            },
        }
    }

    #[test]
    fn unknown_probe_fails_down_to_a_small_baseline() {
        let profile = select_quality_profile(GraphicsCapabilitySnapshot::default());
        assert_eq!(profile.tier, QualityTier::Conservative);
        assert_eq!(profile.render_scale_bps, 5_000);
        assert!(!profile.shadows_enabled);
        assert!(!profile.ambient_occlusion_enabled);
        assert!(!profile.bloom_enabled);
        assert_eq!(profile.texture_residency_bytes, 64 * 1_048_576);
        assert!(!profile.hdr_enabled);
    }

    #[test]
    fn confirmed_features_are_admitted_with_tier_budgets() {
        let profile = select_quality_profile(capable(QualityTier::High));
        assert_eq!(profile.render_scale_bps, 10_000);
        assert!(profile.shadows_enabled);
        assert_eq!(profile.shadow_map_dimension, 4_096);
        assert_eq!(profile.shadow_cascade_count, 4);
        assert!(profile.ambient_occlusion_enabled);
        assert_eq!(profile.ao_scale_bps, 8_500);
        assert!(profile.bloom_enabled);
        assert_eq!(profile.bloom_levels, 5);
        assert!(profile.hdr_enabled && profile.volume_projection_enabled);
        assert_eq!(profile.texture_residency_bytes, 2_048 * 1_048_576);
    }

    #[test]
    fn limits_and_unconfirmed_features_degrade_independently() {
        let mut snapshot = capable(QualityTier::Ultra);
        snapshot.max_texture_dimension_2d = Some(2_048);
        snapshot.confirmed_features = feature::SHADOWS | feature::BLOOM;
        let profile = select_quality_profile(snapshot);
        assert!(!profile.shadows_enabled); // 8192 map exceeds adapter limit
        assert_eq!(profile.shadow_map_dimension, 0);
        assert!(!profile.ambient_occlusion_enabled);
        assert!(profile.bloom_enabled);
        assert!(!profile.hdr_enabled);
        assert!(!profile.volume_projection_enabled);
    }

    #[test]
    fn memory_budget_is_capped_by_tier_and_zero_is_honored() {
        let mut snapshot = capable(QualityTier::Balanced);
        snapshot.gpu_memory_budget_bytes = Some(u64::MAX);
        assert_eq!(
            select_quality_profile(snapshot).texture_residency_bytes,
            1_024 * 1_048_576
        );
        snapshot.gpu_memory_budget_bytes = Some(0);
        assert_eq!(select_quality_profile(snapshot).texture_residency_bytes, 0);
    }

    #[test]
    fn incomplete_probe_never_trusts_high_tier_hint() {
        let mut snapshot = capable(QualityTier::Ultra);
        snapshot.probe_complete = false;
        let profile = select_quality_profile(snapshot);
        assert_eq!(profile.tier, QualityTier::Conservative);
        assert!(!profile.shadows_enabled);
        assert!(!profile.bloom_enabled);
    }

    #[test]
    fn complete_probe_without_performance_evidence_stays_conservative() {
        let mut snapshot = capable(QualityTier::High);
        snapshot.acceptance = Default::default();
        let profile = select_quality_profile(snapshot);
        assert_eq!(profile.tier, QualityTier::Conservative);
        assert_eq!(snapshot.quality_tier_gate(), CapabilityEvidence::Unknown);
    }

    #[test]
    fn vibescript_selects_shared_tiers_and_fails_partial_probes_conservative() {
        let mut snapshot = capable(QualityTier::High);
        assert_eq!(select_quality_tier_via_vibe(snapshot), QualityTier::High);
        snapshot.tier_hint = Some(QualityTier::Ultra);
        assert_eq!(select_quality_tier_via_vibe(snapshot), QualityTier::Ultra);
        snapshot.probe_complete = false;
        assert_eq!(
            select_quality_tier_via_vibe(snapshot),
            QualityTier::Conservative
        );
    }
}
