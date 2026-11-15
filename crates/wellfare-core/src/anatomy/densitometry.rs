//! Skeletal and muscular densitometry, fracture callus healing, and coupled
//! mobility / fracture-risk evaluation across human clinical and comparative
//! biological taxa (Spec §5.1–5.4, BIO-WP-05).
//!
//! **Honesty boundaries:**
//! - Computed biomechanical metrics (`MobilityAndFractureRiskReport`, modulus,
//!   yield strength) always carry [`EpistemicStatus::Hypothesis`], never a
//!   clinical diagnosis.
//! - Missing clinical measurements (such as DEXA `T-score`) stay `None`; WHO
//!   classification returns `None` rather than fabricating a score.
//! - Pure calculations mirror the closed-form equations in
//!   `qualia_core_db::specialized_libs::medical_computing::clinical_formulas`
//!   without pulling `qualia-core-db` (which would introduce a Cargo cycle).

use serde::{Deserialize, Serialize};

use crate::record::EpistemicStatus;

/// WHO (1994) diagnostic classification for areal bone mineral density (DEXA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhoDensityCategory {
    /// T-score >= -1.0.
    Normal,
    /// -2.5 < T-score < -1.0 (low bone mass).
    Osteopenia,
    /// T-score <= -2.5 without fragility fracture.
    Osteoporosis,
    /// T-score <= -2.5 with one or more fragility fractures.
    SevereOsteoporosis,
}

/// Ontogenetic epiphyseal growth-plate (physis) fusion status (Spec §5.5, §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysealFusionStatus {
    OpenCartilaginousPhysis,
    PartiallyFused,
    ClosedMatureDiaphysis,
}

/// Named phase corresponding to the 5 stages (`0..=4`) of [`FractureCallusState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallusHealingPhase {
    /// Stage 0: Unfractured intact structural support element.
    Intact,
    /// Stage 1: Acute fracture hematoma (`E_callus` ~ 0.1–1 MPa; high pain-guarding).
    AcuteHematoma,
    /// Stage 2: Intramembranous & endochondral soft callus (`E_callus` ~ 10–500 MPa).
    SoftCallus,
    /// Stage 3: Mineralizing woven-bone hard callus (`E_callus` ~ 1–5 GPa).
    HardCallus,
    /// Stage 4: Lamellar cortical remodeling (`E` -> 12–20 GPa).
    Remodeling,
}

/// Five-stage (0..=4) mechanobiological fracture-healing state (Claes & Heigele 1999;
/// Spec §5.3 & §9.1).
///
/// - `0` = `Intact`
/// - `1` = `AcuteHematoma`
/// - `2` = `SoftCallus`
/// - `3` = `HardCallus`
/// - `4` = `Remodeling`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FractureCallusState {
    pub stage: u8,
    pub phase: CallusHealingPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_injury_days: Option<f32>,
    /// Effective callus modulus in MPa (`E_callus`).
    pub callus_modulus_mpa: f64,
    /// Fraction of intact structural strength restored in `[0.0, 1.0]`.
    pub strength_recovery_ratio: f64,
    /// Nociceptive pain-guarding coactivation signal in `[0.0, 1.0]` for `aura`.
    pub pain_guarding_coactivation: f64,
}

impl FractureCallusState {
    /// Construct from canonical healing stage `0..=4`. Fails closed for `stage > 4`.
    pub fn from_stage(stage: u8) -> Result<Self, String> {
        Self::from_stage_with_days(stage, None)
    }

    /// Construct from canonical healing stage `0..=4` and optional days post-injury.
    pub fn from_stage_with_days(
        stage: u8,
        post_injury_days: Option<f32>,
    ) -> Result<Self, String> {
        if let Some(d) = post_injury_days {
            if !d.is_finite() || d < 0.0 {
                return Err("post_injury_days must be finite and non-negative".into());
            }
        }
        let (phase, callus_modulus_mpa, strength_recovery_ratio, pain_guarding_coactivation) =
            match stage {
                0 => (CallusHealingPhase::Intact, 16_000.0, 1.00, 0.00),
                1 => (CallusHealingPhase::AcuteHematoma, 0.5, 0.02, 0.85),
                2 => (CallusHealingPhase::SoftCallus, 200.0, 0.15, 0.55),
                3 => (CallusHealingPhase::HardCallus, 2_500.0, 0.65, 0.20),
                4 => (CallusHealingPhase::Remodeling, 15_000.0, 0.95, 0.05),
                _ => {
                    return Err(format!(
                        "invalid fracture callus stage {stage}: expected 0..=4"
                    ));
                }
            };
        Ok(Self {
            stage,
            phase,
            post_injury_days,
            callus_modulus_mpa,
            strength_recovery_ratio,
            pain_guarding_coactivation,
        })
    }

    /// Stage 0: unfractured intact bone.
    pub fn intact() -> Self {
        Self::from_stage(0).expect("stage 0 is valid")
    }

    /// Validate stage consistency and finite/bounded ratios.
    pub fn validate(&self) -> Result<(), String> {
        if self.stage > 4 {
            return Err(format!(
                "invalid fracture callus stage {}: expected 0..=4",
                self.stage
            ));
        }
        if !self.callus_modulus_mpa.is_finite() || self.callus_modulus_mpa < 0.0 {
            return Err("callus_modulus_mpa must be finite and non-negative".into());
        }
        if !self.strength_recovery_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.strength_recovery_ratio)
        {
            return Err("strength_recovery_ratio must be in [0.0, 1.0]".into());
        }
        if !self.pain_guarding_coactivation.is_finite()
            || !(0.0..=1.0).contains(&self.pain_guarding_coactivation)
        {
            return Err("pain_guarding_coactivation must be in [0.0, 1.0]".into());
        }
        if let Some(d) = self.post_injury_days {
            if !d.is_finite() || d < 0.0 {
                return Err("post_injury_days must be finite and non-negative".into());
            }
        }
        Ok(())
    }
}

impl Default for FractureCallusState {
    fn default() -> Self {
        Self::intact()
    }
}

/// Multi-scale cortical and trabecular skeletal densitometry profile (Spec §5.1–5.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletalDensityProfile {
    pub compartment_label: String,
    /// Cortical apparent wet density `rho_app` in g/cm^3.
    pub cortical_apparent_density_g_cm3: f64,
    /// Cortical shell thickness `t_cort` in mm.
    pub cortical_thickness_mm: f64,
    /// Intracortical Haversian porosity `phi_cort` in `[0.0, 1.0]`.
    pub cortical_porosity_ratio: f64,
    /// Trabecular apparent density `rho_app` in g/cm^3.
    pub trabecular_apparent_density_g_cm3: f64,
    /// Trabecular Bone Volume Fraction `BV/TV` in `[0.0, 1.0]`.
    pub trabecular_bv_tv: f64,
    /// Trabecular thickness `Tb.Th` in micrometers.
    pub trabecular_thickness_um: f64,
    /// Trabecular separation `Tb.Sp` in micrometers.
    pub trabecular_separation_um: f64,
    /// Trabecular number `Tb.N` in mm^-1.
    pub trabecular_number_per_mm: f64,
    /// Structure Model Index `SMI` (0 = plate-like, 3 = rod-like).
    pub trabecular_smi: f64,
    /// Principal eigenvalues `[m1, m2, m3]` of the trabecular fabric tensor (`tr(M) = 3.0`).
    pub fabric_eigenvalues: [f64; 3],
    /// Matrix mineralization degree `mu_min` (`1.0` = healthy mineralized human bone;
    /// `< 0.75` = osteomalacic/rachitic hypomineralization).
    pub mineralization_degree_mu: f64,
    /// Mineral ash fraction `alpha_ash` in `[0.0, 1.0]`.
    pub ash_fraction: f64,
    /// QCT / HR-pQCT volumetric BMD in mg HA/cm^3, if observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qct_volumetric_bmd_mg_ha_cm3: Option<f64>,
    /// DEXA areal BMD in g/cm^2, if observed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dexa_areal_bmd_g_cm2: Option<f64>,
    /// DEXA T-score (SD vs young-adult peak), if observed/computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dexa_t_score: Option<f64>,
    /// DEXA Z-score (SD vs age/sex/cohort reference), if observed/computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dexa_z_score: Option<f64>,
    /// Trabecular Bone Score (TBS), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trabecular_bone_score_tbs: Option<f64>,
    pub has_fragility_fracture: bool,
    pub physeal_fusion: PhysealFusionStatus,
    pub fracture_callus: FractureCallusState,
    pub has_orthopedic_implant: bool,
    pub epistemic_status: EpistemicStatus,
}

/// Alias matching the name referenced in Spec §12.3 (`BIO-WP-05`).
pub type SkeletalDensitometryProfile = SkeletalDensityProfile;

impl SkeletalDensityProfile {
    /// Construct a profile from one of the curated human clinical or comparative species presets.
    pub fn from_preset(preset: SpeciesDensitometryPreset) -> Self {
        preset.to_profile()
    }

    /// Fail-closed validation of physical bounds and finite numbers.
    pub fn validate(&self) -> Result<(), String> {
        if self.compartment_label.trim().is_empty() {
            return Err("compartment_label must not be empty".into());
        }
        let non_neg_finite = [
            (
                "cortical_apparent_density_g_cm3",
                self.cortical_apparent_density_g_cm3,
            ),
            ("cortical_thickness_mm", self.cortical_thickness_mm),
            (
                "trabecular_apparent_density_g_cm3",
                self.trabecular_apparent_density_g_cm3,
            ),
            ("trabecular_thickness_um", self.trabecular_thickness_um),
            ("trabecular_separation_um", self.trabecular_separation_um),
            ("trabecular_number_per_mm", self.trabecular_number_per_mm),
            ("mineralization_degree_mu", self.mineralization_degree_mu),
        ];
        for (name, v) in non_neg_finite {
            if !v.is_finite() || v < 0.0 {
                return Err(format!("{name} must be finite and non-negative"));
            }
        }
        if !self.cortical_porosity_ratio.is_finite()
            || !(0.0..=1.0).contains(&self.cortical_porosity_ratio)
        {
            return Err("cortical_porosity_ratio must be in [0.0, 1.0]".into());
        }
        if !self.trabecular_bv_tv.is_finite() || !(0.0..=1.0).contains(&self.trabecular_bv_tv) {
            return Err("trabecular_bv_tv must be in [0.0, 1.0]".into());
        }
        if !self.ash_fraction.is_finite() || !(0.0..=1.0).contains(&self.ash_fraction) {
            return Err("ash_fraction must be in [0.0, 1.0]".into());
        }
        if !self.trabecular_smi.is_finite() {
            return Err("trabecular_smi must be finite".into());
        }
        let trace: f64 = self.fabric_eigenvalues.iter().sum();
        if self
            .fabric_eigenvalues
            .iter()
            .any(|m| !m.is_finite() || *m <= 0.0)
            || (trace - 3.0).abs() > 0.05
        {
            return Err("fabric_eigenvalues must be positive and sum to ~3.0".into());
        }
        if let Some(t) = self.dexa_t_score {
            if !t.is_finite() {
                return Err("dexa_t_score must be finite".into());
            }
        }
        if let Some(z) = self.dexa_z_score {
            if !z.is_finite() {
                return Err("dexa_z_score must be finite".into());
            }
        }
        if let Some(abmd) = self.dexa_areal_bmd_g_cm2 {
            if !abmd.is_finite() || abmd < 0.0 {
                return Err("dexa_areal_bmd_g_cm2 must be finite and non-negative".into());
            }
        }
        if let Some(vbmd) = self.qct_volumetric_bmd_mg_ha_cm3 {
            if !vbmd.is_finite() || vbmd < 0.0 {
                return Err("qct_volumetric_bmd_mg_ha_cm3 must be finite and non-negative".into());
            }
        }
        self.fracture_callus.validate()
    }

    /// WHO (1994) classification from `dexa_t_score` + `has_fragility_fracture`.
    /// Returns `None` if no T-score was measured/provided (never fabricates a score).
    pub fn who_category(&self) -> Option<WhoDensityCategory> {
        let t = self.dexa_t_score?;
        if !t.is_finite() {
            return None;
        }
        Some(if t >= -1.0 {
            WhoDensityCategory::Normal
        } else if t > -2.5 {
            WhoDensityCategory::Osteopenia
        } else if self.has_fragility_fracture {
            WhoDensityCategory::SevereOsteoporosis
        } else {
            WhoDensityCategory::Osteoporosis
        })
    }

    /// Carter & Hayes (1977) / Zysset (2003) effective cortical elastic modulus in GPa,
    /// attenuated by fracture-callus stage when `fracture_callus.stage > 0`.
    pub fn effective_cortical_modulus_gpa(&self, strain_rate_per_s: f64) -> Result<f64, String> {
        self.validate()?;
        if !strain_rate_per_s.is_finite() || strain_rate_per_s <= 0.0 {
            return Err("strain_rate_per_s must be finite and positive".into());
        }
        let m_primary = self.fabric_eigenvalues[0];
        let intact_gpa = 3.790
            * self.mineralization_degree_mu
            * self.cortical_apparent_density_g_cm3.powi(3)
            * m_primary.powi(2)
            * strain_rate_per_s.powf(0.06);
        if self.fracture_callus.stage == 0 {
            Ok(intact_gpa)
        } else {
            Ok(intact_gpa.min(self.fracture_callus.callus_modulus_mpa / 1000.0))
        }
    }

    /// Gibson & Ashby (1997) effective trabecular yield strength in MPa, scaled by
    /// mineralization `mu_min` and fracture-callus strength recovery ratio.
    pub fn effective_trabecular_yield_mpa(&self) -> Result<f64, String> {
        self.validate()?;
        const SOLID_BONE_DENSITY_G_CM3: f64 = 1.85;
        const SOLID_BONE_YIELD_MPA: f64 = 140.0;
        let rel = (self.trabecular_apparent_density_g_cm3 / SOLID_BONE_DENSITY_G_CM3).max(0.0);
        let intact_yield =
            0.3 * SOLID_BONE_YIELD_MPA * self.mineralization_degree_mu * rel.powf(1.5);
        Ok(intact_yield * self.fracture_callus.strength_recovery_ratio)
    }
}

/// Curated comparative and human clinical densitometry presets (Spec §5.2–5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeciesDensitometryPreset {
    HumanYoungAdultFemale,
    HumanPostmenopausalOsteoporotic,
    AvianPneumaticFemur,
    SirenianPachyosteoscleroticRib,
    CetaceanOsteoporoticSpongiosa,
    ElasmobranchTessellatedCartilage,
    CrustaceanCalcifiedCuticle,
    HardwoodSecondaryXylem,
}

impl SpeciesDensitometryPreset {
    /// All 8 comparative and clinical presets.
    pub const ALL: &'static [SpeciesDensitometryPreset] = &[
        Self::HumanYoungAdultFemale,
        Self::HumanPostmenopausalOsteoporotic,
        Self::AvianPneumaticFemur,
        Self::SirenianPachyosteoscleroticRib,
        Self::CetaceanOsteoporoticSpongiosa,
        Self::ElasmobranchTessellatedCartilage,
        Self::CrustaceanCalcifiedCuticle,
        Self::HardwoodSecondaryXylem,
    ];

    /// Materialize the canonical prior [`SkeletalDensityProfile`] for this preset.
    pub fn to_profile(self) -> SkeletalDensityProfile {
        match self {
            Self::HumanYoungAdultFemale => SkeletalDensityProfile {
                compartment_label: "human_young_adult_female_femoral_neck".into(),
                cortical_apparent_density_g_cm3: 1.85,
                cortical_thickness_mm: 4.2,
                cortical_porosity_ratio: 0.04,
                trabecular_apparent_density_g_cm3: 0.32,
                trabecular_bv_tv: 0.24,
                trabecular_thickness_um: 165.0,
                trabecular_separation_um: 580.0,
                trabecular_number_per_mm: 1.45,
                trabecular_smi: 0.9,
                fabric_eigenvalues: [1.15, 0.95, 0.90],
                mineralization_degree_mu: 1.00,
                ash_fraction: 0.65,
                qct_volumetric_bmd_mg_ha_cm3: Some(310.0),
                dexa_areal_bmd_g_cm2: Some(1.02),
                dexa_t_score: Some(0.17),
                dexa_z_score: Some(0.17),
                trabecular_bone_score_tbs: Some(1.42),
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::HumanPostmenopausalOsteoporotic => SkeletalDensityProfile {
                compartment_label: "human_postmenopausal_osteoporotic_femoral_neck".into(),
                cortical_apparent_density_g_cm3: 1.62,
                cortical_thickness_mm: 2.3,
                cortical_porosity_ratio: 0.14,
                trabecular_apparent_density_g_cm3: 0.15,
                trabecular_bv_tv: 0.11,
                trabecular_thickness_um: 112.0,
                trabecular_separation_um: 890.0,
                trabecular_number_per_mm: 0.98,
                trabecular_smi: 2.3,
                fabric_eigenvalues: [1.22, 0.92, 0.86],
                mineralization_degree_mu: 0.96,
                ash_fraction: 0.62,
                qct_volumetric_bmd_mg_ha_cm3: Some(145.0),
                dexa_areal_bmd_g_cm2: Some(0.64),
                dexa_t_score: Some(-3.00),
                dexa_z_score: Some(-1.40),
                trabecular_bone_score_tbs: Some(1.14),
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::AvianPneumaticFemur => SkeletalDensityProfile {
                compartment_label: "avian_pneumatic_femur".into(),
                cortical_apparent_density_g_cm3: 1.78,
                cortical_thickness_mm: 1.1,
                cortical_porosity_ratio: 0.03,
                trabecular_apparent_density_g_cm3: 0.12,
                trabecular_bv_tv: 0.08,
                trabecular_thickness_um: 140.0,
                trabecular_separation_um: 1400.0,
                trabecular_number_per_mm: 0.60,
                trabecular_smi: 2.1,
                fabric_eigenvalues: [1.40, 0.85, 0.75],
                mineralization_degree_mu: 1.12,
                ash_fraction: 0.72,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::SirenianPachyosteoscleroticRib => SkeletalDensityProfile {
                compartment_label: "sirenian_pachyosteosclerotic_rib".into(),
                cortical_apparent_density_g_cm3: 2.02,
                cortical_thickness_mm: 14.0,
                cortical_porosity_ratio: 0.02,
                trabecular_apparent_density_g_cm3: 1.85,
                trabecular_bv_tv: 0.95,
                trabecular_thickness_um: 480.0,
                trabecular_separation_um: 40.0,
                trabecular_number_per_mm: 2.20,
                trabecular_smi: 0.1,
                fabric_eigenvalues: [1.05, 1.00, 0.95],
                mineralization_degree_mu: 1.04,
                ash_fraction: 0.68,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::CetaceanOsteoporoticSpongiosa => SkeletalDensityProfile {
                compartment_label: "cetacean_osteoporotic_spongiosa".into(),
                cortical_apparent_density_g_cm3: 1.55,
                cortical_thickness_mm: 1.8,
                cortical_porosity_ratio: 0.12,
                trabecular_apparent_density_g_cm3: 0.24,
                trabecular_bv_tv: 0.18,
                trabecular_thickness_um: 135.0,
                trabecular_separation_um: 680.0,
                trabecular_number_per_mm: 1.15,
                trabecular_smi: 1.6,
                fabric_eigenvalues: [1.10, 1.00, 0.90],
                mineralization_degree_mu: 0.95,
                ash_fraction: 0.60,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::ElasmobranchTessellatedCartilage => SkeletalDensityProfile {
                compartment_label: "elasmobranch_tessellated_cartilage".into(),
                cortical_apparent_density_g_cm3: 1.70,
                cortical_thickness_mm: 1.5,
                cortical_porosity_ratio: 0.08,
                trabecular_apparent_density_g_cm3: 1.10,
                trabecular_bv_tv: 0.10,
                trabecular_thickness_um: 220.0,
                trabecular_separation_um: 900.0,
                trabecular_number_per_mm: 0.80,
                trabecular_smi: 1.2,
                fabric_eigenvalues: [1.10, 1.05, 0.85],
                mineralization_degree_mu: 0.85,
                ash_fraction: 0.20,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::OpenCartilaginousPhysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::CrustaceanCalcifiedCuticle => SkeletalDensityProfile {
                compartment_label: "crustacean_calcified_cuticle".into(),
                cortical_apparent_density_g_cm3: 1.65,
                cortical_thickness_mm: 2.2,
                cortical_porosity_ratio: 0.05,
                trabecular_apparent_density_g_cm3: 0.05,
                trabecular_bv_tv: 0.05,
                trabecular_thickness_um: 90.0,
                trabecular_separation_um: 1200.0,
                trabecular_number_per_mm: 0.50,
                trabecular_smi: 1.5,
                fabric_eigenvalues: [1.20, 1.00, 0.80],
                mineralization_degree_mu: 0.90,
                ash_fraction: 0.75,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::ClosedMatureDiaphysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
            Self::HardwoodSecondaryXylem => SkeletalDensityProfile {
                compartment_label: "hardwood_secondary_xylem".into(),
                cortical_apparent_density_g_cm3: 0.78,
                cortical_thickness_mm: 45.0,
                cortical_porosity_ratio: 0.35,
                trabecular_apparent_density_g_cm3: 0.62,
                trabecular_bv_tv: 0.55,
                trabecular_thickness_um: 60.0,
                trabecular_separation_um: 120.0,
                trabecular_number_per_mm: 4.50,
                trabecular_smi: 0.6,
                fabric_eigenvalues: [1.80, 0.60, 0.60],
                mineralization_degree_mu: 0.45,
                ash_fraction: 0.02,
                qct_volumetric_bmd_mg_ha_cm3: None,
                dexa_areal_bmd_g_cm2: None,
                dexa_t_score: None,
                dexa_z_score: None,
                trabecular_bone_score_tbs: None,
                has_fragility_fracture: false,
                physeal_fusion: PhysealFusionStatus::OpenCartilaginousPhysis,
                fracture_callus: FractureCallusState::intact(),
                has_orthopedic_implant: false,
                epistemic_status: EpistemicStatus::Hypothesis,
            },
        }
    }
}

/// Skeletal muscle architectural and myosteatosis quality profile (Spec §5.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MuscleQualityProfile {
    pub muscle_group: String,
    pub volume_cm3: f64,
    pub optimal_fiber_length_cm: f64,
    pub pennation_angle_rad: f64,
    pub pcsa_cm2: f64,
    pub specific_tension_n_cm2: f64,
    /// CT skeletal muscle attenuation in Hounsfield Units (healthy: +40..+60 HU; myosteatosis < +30 HU).
    pub muscle_radiodensity_hu: f64,
    /// Intramuscular / intermuscular adipose fraction (`PDFF` / `IMAT`) in `[0.0, 1.0]`.
    pub imat_fraction: f64,
    /// Neuromuscular activation / neural drive factor in `[0.0, 1.0]`.
    pub neural_activation_factor: f64,
    /// Composite contractile quality multiplier `eta_myo` in `[0.0, 1.0]`.
    pub effective_quality_eta: f64,
    pub epistemic_status: EpistemicStatus,
}

/// Alias matching the name referenced in Spec §12.3 (`BIO-WP-05`).
pub type MyologicalQualityProfile = MuscleQualityProfile;

impl MuscleQualityProfile {
    /// Construct and validate a muscle quality profile, computing `pcsa_cm2` and `effective_quality_eta`.
    pub fn new(
        muscle_group: impl Into<String>,
        volume_cm3: f64,
        optimal_fiber_length_cm: f64,
        pennation_angle_rad: f64,
        specific_tension_n_cm2: f64,
        muscle_radiodensity_hu: f64,
        imat_fraction: f64,
        neural_activation_factor: f64,
    ) -> Result<Self, String> {
        let muscle_group = muscle_group.into();
        if muscle_group.trim().is_empty() {
            return Err("muscle_group must not be empty".into());
        }
        if !volume_cm3.is_finite()
            || !optimal_fiber_length_cm.is_finite()
            || !pennation_angle_rad.is_finite()
            || !specific_tension_n_cm2.is_finite()
            || !muscle_radiodensity_hu.is_finite()
            || !imat_fraction.is_finite()
            || !neural_activation_factor.is_finite()
            || volume_cm3 < 0.0
            || optimal_fiber_length_cm <= 0.0
            || !(0.0..std::f64::consts::FRAC_PI_2).contains(&pennation_angle_rad)
            || specific_tension_n_cm2 <= 0.0
            || !(-1000.0..=3000.0).contains(&muscle_radiodensity_hu)
            || !(0.0..=1.0).contains(&imat_fraction)
            || !(0.0..=1.0).contains(&neural_activation_factor)
        {
            return Err("invalid muscle architecture or myosteatosis parameter".into());
        }
        let pcsa_cm2 = (volume_cm3 * pennation_angle_rad.cos()) / optimal_fiber_length_cm;
        let hu_quality = ((muscle_radiodensity_hu + 20.0) / 60.0).clamp(0.0, 1.0);
        let effective_quality_eta = (1.0 - imat_fraction) * hu_quality * neural_activation_factor;
        Ok(Self {
            muscle_group,
            volume_cm3,
            optimal_fiber_length_cm,
            pennation_angle_rad,
            pcsa_cm2,
            specific_tension_n_cm2,
            muscle_radiodensity_hu,
            imat_fraction,
            neural_activation_factor,
            effective_quality_eta,
            epistemic_status: EpistemicStatus::Hypothesis,
        })
    }

    /// Recompute `effective_quality_eta` after field modification (e.g., physiological modifier).
    pub fn recompute_quality_eta(&mut self) {
        let hu_quality = ((self.muscle_radiodensity_hu + 20.0) / 60.0).clamp(0.0, 1.0);
        self.effective_quality_eta =
            (1.0 - self.imat_fraction.clamp(0.0, 1.0)) * hu_quality * self.neural_activation_factor.clamp(0.0, 1.0);
    }

    /// Peak isometric force in Newtons along the tendon line of action:
    /// `F_max = PCSA * sigma_0 * cos(alpha) * eta_myo`.
    pub fn effective_max_force_n(&self) -> Result<f64, String> {
        if !self.pcsa_cm2.is_finite()
            || !self.specific_tension_n_cm2.is_finite()
            || !self.pennation_angle_rad.is_finite()
            || !self.effective_quality_eta.is_finite()
            || self.pcsa_cm2 < 0.0
            || self.specific_tension_n_cm2 <= 0.0
            || !(0.0..std::f64::consts::FRAC_PI_2).contains(&self.pennation_angle_rad)
            || !(0.0..=1.0).contains(&self.effective_quality_eta)
        {
            return Err("invalid MuscleQualityProfile parameters".into());
        }
        Ok(self.pcsa_cm2
            * self.specific_tension_n_cm2
            * self.pennation_angle_rad.cos()
            * self.effective_quality_eta)
    }

    /// True when radiodensity falls below +30 HU or IMAT >= 15%.
    pub fn is_myosteatotic(&self) -> bool {
        self.muscle_radiodensity_hu < 30.0 || self.imat_fraction >= 0.15
    }
}

/// Coupled functional mobility, effort reserve, balance stability, and bone fracture-risk report
/// (Spec §5.4). Always [`EpistemicStatus::Hypothesis`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobilityAndFractureRiskReport {
    pub task_label: String,
    /// Joint Relative Effort Ratio `RER = |tau_required| / |tau_max|` (`>= 1.0` exceeds reserve).
    pub joint_relative_effort_ratio: f64,
    /// Hof (2005) Margin of Stability `MoS = bos_boundary - XCoM` in meters (`< 0` = unstable).
    pub hof_margin_of_stability_m: f64,
    /// Vertebral compressive Factor of Risk `Phi_vertebra = sigma_applied / sigma_yield`.
    pub vertebral_factor_of_risk_phi: f64,
    /// Femoral neck / hip Factor of Risk `Phi_hip = sigma_applied / sigma_yield`.
    pub femoral_neck_factor_of_risk_phi: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who_category: Option<WhoDensityCategory>,
    pub myosteatosis_present: bool,
    pub limiting_factors: Vec<String>,
    pub epistemic_status: EpistemicStatus,
    pub uncertainty_note: String,
}

/// Kinematic & external-load task inputs for [`evaluate_mobility_and_fracture_risk`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobilityTaskInput {
    pub task_label: String,
    pub required_joint_torque_nm: f64,
    pub muscle_moment_arm_m: f64,
    pub com_pos_m: f64,
    pub com_vel_m_s: f64,
    pub pendulum_length_m: f64,
    pub bos_boundary_m: f64,
    pub gravity_m_s2: f64,
    pub applied_vertebral_stress_mpa: f64,
    pub applied_femoral_neck_stress_mpa: f64,
}

/// Evaluate coupled mobility effort (`RER`), dynamic balance (`MoS`), and skeletal
/// factor-of-risk (`Phi`) from calibrated skeletal and muscle quality profiles.
pub fn evaluate_mobility_and_fracture_risk(
    bone: &SkeletalDensityProfile,
    muscle: &MuscleQualityProfile,
    task: &MobilityTaskInput,
) -> Result<MobilityAndFractureRiskReport, String> {
    bone.validate()?;
    if task.task_label.trim().is_empty() {
        return Err("task_label must not be empty".into());
    }
    if !task.required_joint_torque_nm.is_finite()
        || !task.muscle_moment_arm_m.is_finite()
        || !task.com_pos_m.is_finite()
        || !task.com_vel_m_s.is_finite()
        || !task.pendulum_length_m.is_finite()
        || !task.bos_boundary_m.is_finite()
        || !task.gravity_m_s2.is_finite()
        || !task.applied_vertebral_stress_mpa.is_finite()
        || !task.applied_femoral_neck_stress_mpa.is_finite()
        || task.muscle_moment_arm_m <= 0.0
        || task.pendulum_length_m <= 0.0
        || task.gravity_m_s2 <= 0.0
        || task.applied_vertebral_stress_mpa < 0.0
        || task.applied_femoral_neck_stress_mpa < 0.0
    {
        return Err("invalid kinematic or stress task parameters".into());
    }

    let f_max = muscle.effective_max_force_n()?;
    let max_torque_nm = f_max * task.muscle_moment_arm_m;
    if max_torque_nm <= 0.0 {
        return Err("effective max joint torque must be positive to compute RER".into());
    }

    let joint_relative_effort_ratio = task.required_joint_torque_nm.abs() / max_torque_nm;
    let omega0 = (task.gravity_m_s2 / task.pendulum_length_m).sqrt();
    let xcom = task.com_pos_m + task.com_vel_m_s / omega0;
    let hof_margin_of_stability_m = task.bos_boundary_m - xcom;

    let yield_mpa = bone.effective_trabecular_yield_mpa()?;
    if yield_mpa <= 0.0 {
        return Err("effective trabecular yield strength must be positive".into());
    }
    let vertebral_factor_of_risk_phi = task.applied_vertebral_stress_mpa / yield_mpa;
    let femoral_neck_factor_of_risk_phi = task.applied_femoral_neck_stress_mpa / yield_mpa;

    let who_category = bone.who_category();
    let myosteatosis_present = muscle.is_myosteatotic();
    let mut limiting_factors = Vec::new();
    if joint_relative_effort_ratio >= 0.85 {
        limiting_factors.push(format!(
            "elevated relative effort on {} (RER={joint_relative_effort_ratio:.2})",
            muscle.muscle_group
        ));
    }
    if hof_margin_of_stability_m < 0.0 {
        limiting_factors.push(format!(
            "negative Hof margin of stability (MoS={hof_margin_of_stability_m:.3} m)"
        ));
    }
    if vertebral_factor_of_risk_phi >= 1.0 {
        limiting_factors.push(format!(
            "vertebral factor of risk >= 1.0 (Phi={vertebral_factor_of_risk_phi:.2})"
        ));
    }
    if femoral_neck_factor_of_risk_phi >= 1.0 {
        limiting_factors.push(format!(
            "femoral neck factor of risk >= 1.0 (Phi={femoral_neck_factor_of_risk_phi:.2})"
        ));
    }
    if myosteatosis_present {
        limiting_factors.push("myosteatosis present (reduced contractile quality)".into());
    }
    if bone.fracture_callus.stage > 0 && bone.fracture_callus.stage < 4 {
        limiting_factors.push(format!(
            "active fracture callus healing stage {}",
            bone.fracture_callus.stage
        ));
    }

    Ok(MobilityAndFractureRiskReport {
        task_label: task.task_label.clone(),
        joint_relative_effort_ratio,
        hof_margin_of_stability_m,
        vertebral_factor_of_risk_phi,
        femoral_neck_factor_of_risk_phi,
        who_category,
        myosteatosis_present,
        limiting_factors,
        epistemic_status: EpistemicStatus::Hypothesis,
        uncertainty_note: "Coupled biomechanical hypothesis derived from calibrated density/muscle inputs; not a clinical diagnosis.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_eight_species_presets_validate_and_order_physically() {
        for preset in SpeciesDensitometryPreset::ALL {
            let p = SkeletalDensityProfile::from_preset(*preset);
            p.validate().expect("preset must validate");
        }
        let young = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::HumanYoungAdultFemale,
        );
        let osteo = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::HumanPostmenopausalOsteoporotic,
        );
        let sirenian = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::SirenianPachyosteoscleroticRib,
        );
        let cetacean = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::CetaceanOsteoporoticSpongiosa,
        );

        assert_eq!(young.who_category(), Some(WhoDensityCategory::Normal));
        assert_eq!(osteo.who_category(), Some(WhoDensityCategory::Osteoporosis));
        assert_eq!(sirenian.who_category(), None);

        let e_sir = sirenian.effective_cortical_modulus_gpa(0.01).unwrap();
        let e_cet = cetacean.effective_cortical_modulus_gpa(0.01).unwrap();
        assert!(e_sir > e_cet);
        assert!(
            young.effective_trabecular_yield_mpa().unwrap()
                > osteo.effective_trabecular_yield_mpa().unwrap()
        );
    }

    #[test]
    fn fracture_callus_stages_0_through_4_progress_and_reject_out_of_bounds() {
        assert!(FractureCallusState::from_stage(5).is_err());
        let s0 = FractureCallusState::from_stage(0).unwrap();
        let s1 = FractureCallusState::from_stage(1).unwrap();
        let s2 = FractureCallusState::from_stage(2).unwrap();
        let s3 = FractureCallusState::from_stage(3).unwrap();
        let s4 = FractureCallusState::from_stage(4).unwrap();
        assert_eq!(s0.phase, CallusHealingPhase::Intact);
        assert_eq!(s1.phase, CallusHealingPhase::AcuteHematoma);
        assert_eq!(s2.phase, CallusHealingPhase::SoftCallus);
        assert_eq!(s3.phase, CallusHealingPhase::HardCallus);
        assert_eq!(s4.phase, CallusHealingPhase::Remodeling);
        assert!(s1.callus_modulus_mpa < s2.callus_modulus_mpa);
        assert!(s2.callus_modulus_mpa < s3.callus_modulus_mpa);
        assert!(s3.callus_modulus_mpa < s4.callus_modulus_mpa);
        assert!(s1.pain_guarding_coactivation > s4.pain_guarding_coactivation);
    }

    #[test]
    fn coupled_osteosarcopenia_raises_effort_and_fracture_risk() {
        let young_bone = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::HumanYoungAdultFemale,
        );
        let osteo_bone = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::HumanPostmenopausalOsteoporotic,
        );
        let healthy_muscle =
            MuscleQualityProfile::new("quadriceps", 420.0, 10.0, 0.24, 40.0, 52.0, 0.03, 1.0)
                .unwrap();
        let myo_muscle =
            MuscleQualityProfile::new("quadriceps", 300.0, 10.0, 0.24, 40.0, 18.0, 0.24, 0.85)
                .unwrap();

        assert!(!healthy_muscle.is_myosteatotic());
        assert!(myo_muscle.is_myosteatotic());

        let task = MobilityTaskInput {
            task_label: "sit_to_stand".into(),
            required_joint_torque_nm: 65.0,
            muscle_moment_arm_m: 0.05,
            com_pos_m: 0.04,
            com_vel_m_s: 0.15,
            pendulum_length_m: 0.95,
            bos_boundary_m: 0.14,
            gravity_m_s2: 9.81,
            applied_vertebral_stress_mpa: 1.8,
            applied_femoral_neck_stress_mpa: 2.2,
        };

        let r_young = evaluate_mobility_and_fracture_risk(&young_bone, &healthy_muscle, &task).unwrap();
        let r_osteo = evaluate_mobility_and_fracture_risk(&osteo_bone, &myo_muscle, &task).unwrap();

        assert_eq!(r_young.epistemic_status, EpistemicStatus::Hypothesis);
        assert!(r_osteo.joint_relative_effort_ratio > r_young.joint_relative_effort_ratio);
        assert!(r_osteo.femoral_neck_factor_of_risk_phi > r_young.femoral_neck_factor_of_risk_phi);
        assert!(r_osteo.myosteatosis_present);
    }

    #[test]
    fn missing_t_score_never_fabricates_who_category() {
        let mut p = SkeletalDensityProfile::from_preset(
            SpeciesDensitometryPreset::HumanYoungAdultFemale,
        );
        p.dexa_t_score = None;
        assert_eq!(p.who_category(), None);
    }
}
