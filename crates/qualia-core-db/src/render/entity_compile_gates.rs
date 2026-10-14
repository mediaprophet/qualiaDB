//! Deterministic Validation Gates & TAA Oklab Clamping for `qualia-cli entity-compile`.
//!
//! Enforces the compilation gates before sealing `.10d` living containers
//! and packaging into `.hmc` bundles:
//!
//! - **Gate 1: Spectral Plausibility & Energy Conservation** ($R(\lambda) \in [0.0, 1.0]$ across 380–780 nm).
//! - **Gate 2: Virtualized Geometry Confinement** (Tier 2/3/4 strictly bounded inside Tier 1 AABB).
//! - **Gate 3: Ontological & Anti-Commodification Integrity** (Valid ontological class, anti-slavery /
//!   inalienable human rights protection preventing `NaturalPerson` from being commodified as an asset).
//! - **Gate 4: Wasm/VibeScript Determinism Limits** (Zero non-deterministic host calls, cycle count budget).
//!
//! Also provides perceptual **Oklab Neighborhood Clamping** for Temporal Anti-Aliasing (TAA)
//! across blue-noise hero wavelength packets.

use crate::render::spectral_basis::{
    evaluate_spectral_reflectance, JakobHanikaCoeffs, LAMBDA_MIN_NM,
};

/// First-class ontological categories determining entity representation and rights.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OntologicalClass {
    /// Living organism or evolutionary species (plants, animals, fungi) -> Specimen.
    DarwinianTaxon = 1,
    /// Fabricated machinery, tools, vehicles, infrastructure -> Construct / Manufactured Property.
    ManufacturedProperty = 2,
    /// Authored cultural expressions, stories, music, art -> Creative Work / Expression.
    LiberalArtsWork = 3,
    /// Geological formations, water catchments, abiotic features -> Terrain / Feature.
    AbioticEnvironment = 4,
    /// Human subject with inalienable sovereignty -> Persona / Likeness (CANNOT be owned as an asset).
    NaturalPerson = 5,
}

impl OntologicalClass {
    pub fn from_u8(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::DarwinianTaxon),
            2 => Some(Self::ManufacturedProperty),
            3 => Some(Self::LiberalArtsWork),
            4 => Some(Self::AbioticEnvironment),
            5 => Some(Self::NaturalPerson),
            _ => None,
        }
    }

    pub fn from_str_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "darwiniantaxon" | "taxon" | "specimen" | "species" | "flora" | "fauna" => {
                Some(Self::DarwinianTaxon)
            }
            "manufacturedproperty" | "property" | "construct" | "machine" | "tool" | "asset" => {
                Some(Self::ManufacturedProperty)
            }
            "liberalartswork" | "cultural" | "work" | "expression" | "lore" => {
                Some(Self::LiberalArtsWork)
            }
            "abioticenvironment" | "environment" | "terrain" | "feature" | "geology" => {
                Some(Self::AbioticEnvironment)
            }
            "naturalperson" | "person" | "persona" | "likeness" | "human" => {
                Some(Self::NaturalPerson)
            }
            _ => None,
        }
    }

    /// English noun describing the sensory embodiment in accordance with its ontological realm.
    pub fn embodiment_term(&self) -> &'static str {
        match self {
            Self::DarwinianTaxon => "Specimen",
            Self::ManufacturedProperty => "Construct",
            Self::LiberalArtsWork => "Expression",
            Self::AbioticEnvironment => "Terrain",
            Self::NaturalPerson => "Persona",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateError {
    /// Spectral reflectance violates physical bounds [0.0, 1.0].
    SpectralEnergyViolation { lambda_nm: u32, reflectance: u32 },
    /// BSSRDF mean free path contains zero or negative values.
    BssrdfZeroOrNegative,
    /// Tier 2/3/4 bounds spill outside Tier 1 Hero AABB.
    GeometryBoundsViolation,
    /// Impostor bounding sphere misaligned with mesh geometry.
    ImpostorSphereMisaligned,
    /// Malformed or unrecognized ontological class.
    MalformedOntologicalClass,
    /// Missing mandatory physical constants (e.g., mass for manufactured machinery).
    MissingPhysicalConstants,
    /// Attempting to commodify or classify a NaturalPerson as ManufacturedProperty or CommercialAsset.
    HumanPersonCannotBeCommodifiedAsAsset,
    /// Affordance script exceeds cycle budget or calls non-deterministic host APIs.
    NonDeterministicScriptExecution,
    /// Living biosphere ("world of god") entities must be licensed under Permissive Commons or Social Permissive Commons.
    BiosphereRequiresPermissiveCommons,
    /// Living biosphere entities cannot be associated with perpetual royalty fees.
    BiosphereCannotHavePerpetualFees,
}

impl core::fmt::Display for GateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SpectralEnergyViolation { lambda_nm, reflectance } => {
                write!(f, "spectral energy violation at {lambda_nm} nm: reflectance {reflectance}")
            }
            Self::BssrdfZeroOrNegative => write!(f, "BSSRDF mean free path must be strictly positive"),
            Self::GeometryBoundsViolation => write!(f, "lower-tier geometry spills outside Tier 1 Hero AABB"),
            Self::ImpostorSphereMisaligned => write!(f, "impostor bounding sphere misaligned with geometry"),
            Self::MalformedOntologicalClass => write!(f, "malformed or missing ontological class"),
            Self::MissingPhysicalConstants => write!(f, "missing mandatory physical constants"),
            Self::HumanPersonCannotBeCommodifiedAsAsset => {
                write!(f, "anti-commodification violation: a NaturalPerson cannot be classified as property or an asset")
            }
            Self::NonDeterministicScriptExecution => write!(f, "script violates determinism or cycle limits"),
            Self::BiosphereRequiresPermissiveCommons => {
                write!(f, "humanitarian ict gate violation: living biosphere entity must be governed under Permissive Commons")
            }
            Self::BiosphereCannotHavePerpetualFees => {
                write!(f, "humanitarian ict gate violation: living biosphere entity cannot be encumbered with perpetual fees")
            }
        }
    }
}

impl std::error::Error for GateError {}

// ───────────────────────────────────────────────────────────────────────────
//  Gate 1: Spectral Plausibility & Energy Conservation
// ───────────────────────────────────────────────────────────────────────────

/// Assert that spectral coefficients evaluate strictly within physical bounds $R(\lambda) \in [0.0, 1.0]$.
pub fn validate_spectral_coeffs(coeffs: &JakobHanikaCoeffs) -> Result<(), GateError> {
    for lambda_step in 0..=40 {
        let lambda_nm = LAMBDA_MIN_NM + (lambda_step as f32) * 10.0;
        let r = evaluate_spectral_reflectance(coeffs, lambda_nm);
        if r < -1e-6 || r > 1.00001 {
            return Err(GateError::SpectralEnergyViolation {
                lambda_nm: lambda_nm as u32,
                reflectance: (r * 1000.0) as u32,
            });
        }
    }
    Ok(())
}

/// Assert that Subsurface Scattering mean free paths are strictly positive (prevents division by zero).
pub fn validate_bssrdf_parameters(mean_free_path_mm: [f32; 4]) -> Result<(), GateError> {
    for &ld in &mean_free_path_mm {
        if ld <= 1e-5 || !ld.is_finite() {
            return Err(GateError::BssrdfZeroOrNegative);
        }
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
//  Gate 2: Virtualized Geometry Confinement
// ───────────────────────────────────────────────────────────────────────────

/// Assert that lower-tier primitives (LODs, surfels) remain strictly bounded inside Tier 1 Hero AABB.
pub fn validate_geometry_confinement(
    hero_min: [f32; 3],
    hero_max: [f32; 3],
    child_min: [f32; 3],
    child_max: [f32; 3],
    sphere_center: [f32; 3],
    sphere_radius: f32,
) -> Result<(), GateError> {
    const EPSILON: f32 = 0.05; // 5 cm margin for decimation drift

    for i in 0..3 {
        if child_min[i] < hero_min[i] - EPSILON || child_max[i] > hero_max[i] + EPSILON {
            return Err(GateError::GeometryBoundsViolation);
        }
    }

    // Verify impostor sphere encompasses the hero AABB center
    let aabb_center = [
        (hero_min[0] + hero_max[0]) * 0.5,
        (hero_min[1] + hero_max[1]) * 0.5,
        (hero_min[2] + hero_max[2]) * 0.5,
    ];
    let dist_sq = (aabb_center[0] - sphere_center[0]).powi(2)
        + (aabb_center[1] - sphere_center[1]).powi(2)
        + (aabb_center[2] - sphere_center[2]).powi(2);

    if dist_sq > (sphere_radius * 0.25).powi(2) {
        return Err(GateError::ImpostorSphereMisaligned);
    }

    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
//  Gate 3 & 4: Ontological Integrity & Anti-Commodification Gate
// ───────────────────────────────────────────────────────────────────────────

/// Assert valid ontological classification and enforce anti-commodification human rights.
pub fn validate_ontological_integrity(
    origin_tag: u8,
    mass_kg: Option<f32>,
) -> Result<(), GateError> {
    match OntologicalClass::from_u8(origin_tag) {
        Some(OntologicalClass::DarwinianTaxon) => Ok(()), // Biological life / species specimen
        Some(OntologicalClass::ManufacturedProperty) => {
            // ManufacturedProperty requires finite mass
            if let Some(m) = mass_kg {
                if m > 0.0 && m.is_finite() {
                    Ok(())
                } else {
                    Err(GateError::MissingPhysicalConstants)
                }
            } else {
                Err(GateError::MissingPhysicalConstants)
            }
        }
        Some(OntologicalClass::LiberalArtsWork) => Ok(()), // Expressive cultural composition
        Some(OntologicalClass::AbioticEnvironment) => Ok(()), // Terrain / environmental commons
        Some(OntologicalClass::NaturalPerson) => Ok(()),   // Sovereign human subject (Persona)
        None => Err(GateError::MalformedOntologicalClass),
    }
}

/// Explicit assertion protecting natural persons from being classified as manufactured property.
pub fn assert_person_anti_commodification(
    entity_class: OntologicalClass,
    as_property: bool,
) -> Result<(), GateError> {
    if entity_class == OntologicalClass::NaturalPerson && as_property {
        Err(GateError::HumanPersonCannotBeCommodifiedAsAsset)
    } else {
        Ok(())
    }
}

/// Assert that living biosphere entities ("world of god") and fundamental knowledge for life
/// are governed by Permissive Commons, preventing enclosure, perpetual royalties, and private monopolization.
///
/// NOTE: Commercial goods (`ManufacturedProperty`) are intentionally unconstrained: commercial vendors
/// (e.g. Nike, toolmakers, device manufacturers) retain commercial rights to set prices and terms.
/// Consumers have a right to fundamental bodily needs (shoes to walk, shelter, clothing), but not
/// an entitlement to a specific commercial brand design. In contrast, living species and knowledge
/// needed for life/freedom of thought belong to the inalienable permissive commons.
pub fn assert_biosphere_permissive_commons(
    entity_class: OntologicalClass,
    licence_tag: &str,
    has_perpetual_royalties: bool,
) -> Result<(), GateError> {
    match entity_class {
        OntologicalClass::DarwinianTaxon | OntologicalClass::AbioticEnvironment => {
            if has_perpetual_royalties {
                return Err(GateError::BiosphereCannotHavePerpetualFees);
            }
            let lic = crate::q42::asset_envelope::LicenceClass::parse(licence_tag);
            match lic {
                crate::q42::asset_envelope::LicenceClass::Cc0
                | crate::q42::asset_envelope::LicenceClass::CcBy
                | crate::q42::asset_envelope::LicenceClass::CcBySa
                | crate::q42::asset_envelope::LicenceClass::SocialPermissiveCommons => Ok(()),
                _ => Err(GateError::BiosphereRequiresPermissiveCommons),
            }
        }
        _ => Ok(()),
    }
}

/// Assert affordance Wasm script determinism.
pub fn validate_script_determinism(
    instruction_count: u64,
    has_unauthorized_host_calls: bool,
) -> Result<(), GateError> {
    const MAX_INSTRUCTION_BUDGET: u64 = 1_000_000;
    if has_unauthorized_host_calls || instruction_count > MAX_INSTRUCTION_BUDGET {
        return Err(GateError::NonDeterministicScriptExecution);
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
//  Temporal Anti-Aliasing (TAA) Oklab Perceptual Clamping
// ───────────────────────────────────────────────────────────────────────────

/// Transform linear CIE XYZ to Oklab perceptual color space. Zero heap.
#[inline]
pub fn xyz_to_oklab(xyz: [f32; 3]) -> [f32; 3] {
    let l = 0.818_933 * xyz[0] + 0.361_866_74 * xyz[1] - 0.128_859_71 * xyz[2];
    let m = 0.032_984_544 * xyz[0] + 0.929_311_9 * xyz[1] + 0.036_145_64 * xyz[2];
    let s = 0.048_200_3 * xyz[0] + 0.264_366_27 * xyz[1] + 0.633_851_7 * xyz[2];

    let l_ = if l > 0.0 { l.cbrt() } else { 0.0 };
    let m_ = if m > 0.0 { m.cbrt() } else { 0.0 };
    let s_ = if s > 0.0 { s.cbrt() } else { 0.0 };

    let l_oklab = 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_;
    let a_oklab = 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_;
    let b_oklab = 0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_;

    [l_oklab, a_oklab, b_oklab]
}

/// Transform Oklab perceptual color space back to linear CIE XYZ. Zero heap.
#[inline]
pub fn oklab_to_xyz(oklab: [f32; 3]) -> [f32; 3] {
    let l_ = oklab[0] + 0.396_337_78 * oklab[1] + 0.215_803_76 * oklab[2];
    let m_ = oklab[0] - 0.105_561_346 * oklab[1] - 0.063_854_17 * oklab[2];
    let s_ = oklab[0] - 0.089_484_18 * oklab[1] - 1.291_485_5 * oklab[2];

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let x = 1.227_013_8 * l - 0.557_797_7 * m + 0.281_256_15 * s;
    let y = -0.040_580_18 * l + 1.112_256_9 * m - 0.071_676_68 * s;
    let z = -0.076_381_284 * l - 0.421_481_98 * m + 1.586_163_2 * s;

    [x, y, z]
}

/// Clamps the reprojected TAA history color to the variance bounding box
/// of the 3x3 local neighborhood in Oklab space. Zero heap.
#[inline]
pub fn clamp_taa_history_oklab(
    history_xyz: [f32; 3],
    neighborhood_3x3_xyz: &[[f32; 3]; 9],
    gamma: f32, // Variance clipping scale, typically 1.0 .. 1.25
) -> [f32; 3] {
    let mut mean = [0.0f32; 3];
    let mut m2 = [0.0f32; 3];

    for s in neighborhood_3x3_xyz.iter() {
        let lab = xyz_to_oklab(*s);
        for c in 0..3 {
            mean[c] += lab[c];
            m2[c] += lab[c] * lab[c];
        }
    }

    let inv_9 = 1.0 / 9.0;
    for c in 0..3 {
        mean[c] *= inv_9;
        m2[c] *= inv_9;
    }

    let mut min_bound = [0.0f32; 3];
    let mut max_bound = [0.0f32; 3];
    for c in 0..3 {
        let variance = (m2[c] - mean[c] * mean[c]).max(0.0);
        let std_dev = variance.sqrt();
        min_bound[c] = mean[c] - gamma * std_dev;
        max_bound[c] = mean[c] + gamma * std_dev;
    }

    let mut hist_lab = xyz_to_oklab(history_xyz);
    for c in 0..3 {
        hist_lab[c] = hist_lab[c].clamp(min_bound[c], max_bound[c]);
    }

    oklab_to_xyz(hist_lab)
}

/// Velocity-ramped dynamic TAA blend factor alpha in [0.05, 0.20].
#[inline]
pub fn calculate_taa_blend_factor(screen_velocity_pixels: f32) -> f32 {
    const ALPHA_STATIC: f32 = 0.05;
    const ALPHA_DYNAMIC_MAX: f32 = 0.20;
    const VELOCITY_RAMP_PIXELS: f32 = 4.0;

    let t = (screen_velocity_pixels / VELOCITY_RAMP_PIXELS).clamp(0.0, 1.0);
    ALPHA_STATIC + t * (ALPHA_DYNAMIC_MAX - ALPHA_STATIC)
}

/// Check relative depth disocclusion gate.
#[inline]
pub fn is_taa_history_disoccluded(current_linear_depth: f32, history_linear_depth: f32) -> bool {
    const DEPTH_THRESHOLD_RATIO: f32 = 0.05; // 5% depth difference trigger
    let diff = (current_linear_depth - history_linear_depth).abs();
    diff > (current_linear_depth * DEPTH_THRESHOLD_RATIO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_commodification_gate_protects_natural_persons() {
        // NaturalPerson cannot be flagged as property/asset
        let err = assert_person_anti_commodification(OntologicalClass::NaturalPerson, true);
        assert_eq!(err, Err(GateError::HumanPersonCannotBeCommodifiedAsAsset));

        // NaturalPerson as Persona is valid
        let ok = assert_person_anti_commodification(OntologicalClass::NaturalPerson, false);
        assert!(ok.is_ok());

        // ManufacturedProperty can be property
        let prop = assert_person_anti_commodification(OntologicalClass::ManufacturedProperty, true);
        assert!(prop.is_ok());
    }

    #[test]
    fn oklab_roundtrip_is_lossless() {
        let white_xyz = [0.950_47, 1.0, 1.088_83];
        let lab = xyz_to_oklab(white_xyz);
        let reconstructed = oklab_to_xyz(lab);

        for c in 0..3 {
            assert!((white_xyz[c] - reconstructed[c]).abs() < 1e-4);
        }
    }

    #[test]
    fn gate1_spectral_validation() {
        // Physically plausible basis coefficients
        let valid_coeffs = JakobHanikaCoeffs {
            c0: 0.5,
            c1: 0.0,
            c2: 0.0,
        }; // Constant 0.5 reflectance
        assert!(validate_spectral_coeffs(&valid_coeffs).is_ok());

        // Non-positive BSSRDF mean free path fails
        let invalid_mfp = [0.0f32, 1.0, 2.0, 1.0];
        assert_eq!(
            validate_bssrdf_parameters(invalid_mfp),
            Err(GateError::BssrdfZeroOrNegative)
        );

        let valid_mfp = [0.5f32, 1.2, 3.4, 1.0];
        assert!(validate_bssrdf_parameters(valid_mfp).is_ok());
    }

    #[test]
    fn gate2_geometry_confinement() {
        let hero_min = [-1.0f32, -1.0, -1.0];
        let hero_max = [1.0f32, 1.0, 1.0];
        let center = [0.0f32, 0.0, 0.0];
        let radius = 1.732f32;

        // Within 5cm margin
        let lod_min = [-1.04f32, -1.0, -1.0];
        let lod_max = [1.04f32, 1.0, 1.0];
        assert!(validate_geometry_confinement(hero_min, hero_max, lod_min, lod_max, center, radius).is_ok());

        // Exceeding margin fails
        let bad_min = [-1.10f32, -1.0, -1.0];
        let bad_max = [1.0f32, 1.0, 1.0];
        assert_eq!(
            validate_geometry_confinement(hero_min, hero_max, bad_min, bad_max, center, radius),
            Err(GateError::GeometryBoundsViolation)
        );
    }

    #[test]
    fn gate3_ontological_integrity() {
        // DarwinianTaxon without mass is valid
        assert!(validate_ontological_integrity(OntologicalClass::DarwinianTaxon as u8, None).is_ok());

        // ManufacturedProperty without mass fails
        assert_eq!(
            validate_ontological_integrity(OntologicalClass::ManufacturedProperty as u8, None),
            Err(GateError::MissingPhysicalConstants)
        );

        // ManufacturedProperty with positive mass is valid
        assert!(validate_ontological_integrity(OntologicalClass::ManufacturedProperty as u8, Some(42.0)).is_ok());

        // Malformed class fails
        assert_eq!(
            validate_ontological_integrity(99, None),
            Err(GateError::MalformedOntologicalClass)
        );
    }

    #[test]
    fn gate4_script_determinism() {
        assert!(validate_script_determinism(500_000, false).is_ok());
        assert_eq!(
            validate_script_determinism(1_500_000, false),
            Err(GateError::NonDeterministicScriptExecution)
        );
        assert_eq!(
            validate_script_determinism(100, true),
            Err(GateError::NonDeterministicScriptExecution)
        );
    }

    #[test]
    fn taa_clamping_and_blend_factor() {
        let center = [0.5f32, 0.5, 0.5];
        let neighborhood = [center; 9];
        let outlier = [1.5f32, 1.5, 1.5];

        let clamped = clamp_taa_history_oklab(outlier, &neighborhood, 1.0);
        assert!(clamped[0] < outlier[0]);

        // Static velocity gives 0.05
        assert_eq!(calculate_taa_blend_factor(0.0), 0.05);
        // High velocity clamps to 0.20
        assert_eq!(calculate_taa_blend_factor(10.0), 0.20);
    }
}
