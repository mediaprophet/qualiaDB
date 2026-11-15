//! Standards-grade metadata schemas (105..=107, 133..=137), rigid/affine frame
//! transforms, calibration windows and spectral response admission for EMF profiles.

use super::spectrum::WorkBudget;
use super::*;

const fn r(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule {
        key,
        type_tag,
        count,
        required,
    }
}

const RULES_105: &[FieldRule] = &[
    r(1, 7, Some(32), true),
    r(2, 6, None, true),
    r(3, 7, Some(32), true),
    r(4, 2, Some(1), true),
    r(5, 8, Some(1), true),
    r(6, 8, Some(1), false),
    r(7, 8, Some(1), false),
    r(8, 6, None, true),
    r(9, 2, None, true),
];

const RULES_106: &[FieldRule] = &[
    r(1, 8, Some(1), true),
    r(2, 8, Some(1), true),
    r(3, 6, None, true),
    r(4, 5, Some(16), true),
    r(5, 8, Some(1), false),
    r(6, 3, Some(2), false),
];

const RULES_107: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 7, Some(32), true),
    r(3, 6, None, true),
    r(4, 5, Some(1), true),
    r(5, 5, Some(1), true),
    r(6, 5, Some(1), false),
    r(7, 6, None, false),
    r(8, 3, Some(2), false),
    r(9, 8, Some(1), false),
];

const RULES_133: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 5, Some(1), true),
    r(3, 5, Some(1), true),
    r(4, 5, Some(1), true),
    r(5, 5, None, false),
    r(6, 5, None, false),
    r(7, 5, None, false),
    r(8, 5, None, false),
    r(9, 9, Some(1), false),
    r(10, 8, Some(1), false),
    r(11, 8, Some(1), false),
];

const RULES_134: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(3, 7, Some(32), true),
    r(4, 6, None, true),
    r(5, 6, None, false),
    r(6, 6, None, false),
    r(7, 6, None, false),
    r(8, 6, None, true),
    r(9, 6, None, false),
    r(10, 3, Some(2), false),
    r(11, 6, None, false),
    r(12, 8, Some(1), false),
];

const RULES_136: &[FieldRule] = &[
    r(1, 8, Some(1), true),
    r(2, 2, Some(1), true),
    r(3, 2, Some(1), true),
    r(4, 6, None, true),
    r(5, 6, None, true),
    r(6, 8, Some(1), true),
    r(7, 6, None, true),
    r(8, 6, None, false),
    r(9, 1, Some(1), false),
    r(10, 1, Some(1), false),
    r(11, 6, None, false),
    r(12, 5, Some(1), false),
    r(13, 1, Some(1), false),
];

const RULES_137: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(3, 6, None, true),
    r(4, 8, Some(1), true),
    r(5, 8, Some(1), false),
    r(6, 8, Some(1), false),
    r(7, 8, Some(1), false),
];

pub fn material_binding_rules() -> &'static [FieldRule] {
    RULES_105
}
pub fn coordinate_transform_rules() -> &'static [FieldRule] {
    RULES_106
}
pub fn calibration_rules() -> &'static [FieldRule] {
    RULES_107
}
pub fn constitutive_model_rules() -> &'static [FieldRule] {
    RULES_133
}
pub fn model_evidence_rules() -> &'static [FieldRule] {
    RULES_134
}
pub fn acquisition_rules() -> &'static [FieldRule] {
    RULES_136
}
pub fn bio_context_rules() -> &'static [FieldRule] {
    RULES_137
}

#[inline]
pub(super) fn i64_at(b: &[u8], at: usize) -> i64 {
    i64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

#[inline]
fn f64_scalar(b: Block<'_>, k: u16) -> Result<f64, Error> {
    let val = f64_at(required(b, k)?.value(), 0);
    if !val.is_finite() {
        return Err(Error::NonFinite);
    }
    Ok(val)
}

fn non_empty_text<'a>(b: Block<'a>, k: u16) -> Result<&'a str, Error> {
    let s = text(required(b, k)?)?;
    if s.is_empty() {
        return Err(Error::Descriptor);
    }
    Ok(s)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstitutiveKind {
    Vacuum,
    IsotropicNondispersive,
    DebyeIsotropic,
    LorentzIsotropic,
    AnisotropicTabulated,
}

impl ConstitutiveKind {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "vacuum" => Ok(Self::Vacuum),
            "isotropic_nondispersive" => Ok(Self::IsotropicNondispersive),
            "debye_isotropic" => Ok(Self::DebyeIsotropic),
            "lorentz_isotropic" => Ok(Self::LorentzIsotropic),
            "anisotropic_tabulated" => Ok(Self::AnisotropicTabulated),
            _ => Err(Error::UnsupportedProfile),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformKind {
    RigidMatrix4,
    AffineMatrix4,
}

impl TransformKind {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "rigid_matrix4" => Ok(Self::RigidMatrix4),
            "affine_matrix4" => Ok(Self::AffineMatrix4),
            _ => Err(Error::UnsupportedProfile),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpistemicClass {
    Empirical,
    ModelEstimate,
    Hypothesis,
    Counterfactual,
    Fictional,
}

impl EpistemicClass {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "empirical" => Ok(Self::Empirical),
            "model_estimate" => Ok(Self::ModelEstimate),
            "hypothesis" => Ok(Self::Hypothesis),
            "counterfactual" => Ok(Self::Counterfactual),
            "fictional" => Ok(Self::Fictional),
            _ => Err(Error::Value),
        }
    }

    pub const fn is_speculative(self) -> bool {
        matches!(
            self,
            Self::Hypothesis | Self::Counterfactual | Self::Fictional
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalKind {
    ComplexBaseband,
    ComplexStft,
    RealSamples,
}

impl SignalKind {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "complex_baseband" => Ok(Self::ComplexBaseband),
            "complex_stft" => Ok(Self::ComplexStft),
            "real_samples" => Ok(Self::RealSamples),
            _ => Err(Error::Value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Rectangular,
    Hann,
}

impl WindowKind {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "rectangular" => Ok(Self::Rectangular),
            "hann" => Ok(Self::Hann),
            _ => Err(Error::Value),
        }
    }
}

use super::frame_transforms::Schema106Transform;

pub fn validate_material_binding(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    if text(required(block, 2)?)? != "BLAKE3-256" {
        return Err(Error::Value);
    }
    let _ = non_empty_text(block, 8)?;
    ctx.require_schema(scalar32(block, 5)?, 2)
        .map_err(Error::Shared)?;
    if let Some(tr) = field(block, 6) {
        ctx.require_schema(u32_at(tr.value(), 0), 106)
            .map_err(Error::Shared)?;
    }
    if let Some(clk) = field(block, 7) {
        ctx.require_schema(u32_at(clk.value(), 0), 1)
            .map_err(Error::Shared)?;
    }
    let ids = required(block, 9)?.value();
    if ids.is_empty() || !ids.len().is_multiple_of(8) {
        return Err(Error::Descriptor);
    }
    let count = ids.len() / 8;
    let work = (count as u64)
        .checked_mul(count as u64)
        .ok_or(Error::Overflow)?;
    ctx.charge_work(work).map_err(Error::Shared)?;
    for i in 0..count {
        let a = u64_at(ids, i * 8);
        for j in (i + 1)..count {
            if a == u64_at(ids, j * 8) {
                return Err(Error::Value);
            }
        }
    }
    Ok(())
}

pub fn material_binding_contains(block: Block<'_>, material_id: u64) -> Result<bool, Error> {
    if block.schema_type() != 105 || block.revision() != 1 {
        return Err(Error::Shared(semantics::Error::ReferenceSchema));
    }
    let ids = required(block, 9)?.value();
    Ok(ids.chunks_exact(8).any(|c| u64_at(c, 0) == material_id))
}

pub fn validate_coordinate_transform(
    block: Block<'_>,
    ctx: &Context<'_, '_>,
) -> Result<(), Error> {
    ctx.require_schema(scalar32(block, 1)?, 2)
        .map_err(Error::Shared)?;
    ctx.require_schema(scalar32(block, 2)?, 2)
        .map_err(Error::Shared)?;
    if let Some(clk) = field(block, 5) {
        ctx.require_schema(u32_at(clk.value(), 0), 1)
            .map_err(Error::Shared)?;
    } else if field(block, 6).is_some() {
        return Err(Error::Time);
    }
    let _ = Schema106Transform::from_block(block)?;
    Ok(())
}

/// Evaluates linear calibration `scale * raw + offset` checking `[from, until)` tick window.
pub fn evaluate_calibration(
    block: Block<'_>,
    raw_value: f64,
    tick: Option<i64>,
) -> Result<f64, Error> {
    if block.schema_type() != 107 || block.revision() != 1 {
        return Err(Error::Shared(semantics::Error::ReferenceSchema));
    }
    if !raw_value.is_finite() {
        return Err(Error::NonFinite);
    }
    let scale = f64_scalar(block, 4)?;
    let offset = f64_scalar(block, 5)?;
    if scale <= 0.0 {
        return Err(Error::Value);
    }
    if let Some(vt) = field(block, 8) {
        let from = i64_at(vt.value(), 0);
        let until = i64_at(vt.value(), 8);
        if from >= until {
            return Err(Error::Time);
        }
        let t = tick.ok_or(Error::Time)?;
        if t < from || t >= until {
            return Err(Error::Time);
        }
    }
    let calibrated = scale * raw_value + offset;
    if !calibrated.is_finite() {
        return Err(Error::NonFinite);
    }
    Ok(calibrated)
}

pub fn validate_calibration(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    let _ = non_empty_text(block, 1)?;
    let _ = non_empty_text(block, 3)?;
    let scale = f64_scalar(block, 4)?;
    let _ = f64_scalar(block, 5)?;
    if scale <= 0.0 {
        return Err(Error::Value);
    }
    if let Some(unc) = field(block, 6) {
        let u = f64_at(unc.value(), 0);
        if !u.is_finite() {
            return Err(Error::NonFinite);
        }
        if u < 0.0 {
            return Err(Error::Value);
        }
        let _ = non_empty_text(block, 7)?;
    } else if field(block, 7).is_some() {
        let _ = non_empty_text(block, 7)?;
    }
    if let Some(clk) = field(block, 9) {
        ctx.require_schema(u32_at(clk.value(), 0), 1)
            .map_err(Error::Shared)?;
    }
    if let Some(vt) = field(block, 8) {
        if field(block, 9).is_none() {
            return Err(Error::Time);
        }
        let from = i64_at(vt.value(), 0);
        let until = i64_at(vt.value(), 8);
        if from >= until {
            return Err(Error::Time);
        }
    }
    Ok(())
}

pub fn validate_constitutive_model(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    let kind = ConstitutiveKind::parse(text(required(block, 1)?)?)?;
    let eps = f64_scalar(block, 2)?;
    let mu = f64_scalar(block, 3)?;
    let sigma = f64_scalar(block, 4)?;
    if eps <= 0.0 || mu <= 0.0 || sigma < 0.0 {
        return Err(Error::Value);
    }
    let f5 = field(block, 5);
    let f6 = field(block, 6);
    let f7 = field(block, 7);
    let f8 = field(block, 8);
    let f9 = field(block, 9);
    match kind {
        ConstitutiveKind::Vacuum => {
            if (eps - 1.0).abs() > 1.0e-12 || (mu - 1.0).abs() > 1.0e-12 || sigma != 0.0 {
                return Err(Error::Value);
            }
            if f5.is_some() || f6.is_some() || f7.is_some() || f8.is_some() || f9.is_some() {
                return Err(Error::Descriptor);
            }
        }
        ConstitutiveKind::IsotropicNondispersive => {
            if f5.is_some() || f6.is_some() || f7.is_some() || f8.is_some() || f9.is_some() {
                return Err(Error::Descriptor);
            }
        }
        ConstitutiveKind::DebyeIsotropic => {
            let de = f5.ok_or(Error::Descriptor)?;
            let tau = f6.ok_or(Error::Descriptor)?;
            if f7.is_some() || f8.is_some() || de.element_count() == 0 {
                return Err(Error::Descriptor);
            }
            if de.element_count() != tau.element_count() {
                return Err(Error::Shape);
            }
            for i in 0..de.element_count() as usize {
                let d = f64_at(de.value(), i * 8);
                let t = f64_at(tau.value(), i * 8);
                if !d.is_finite() || !t.is_finite() {
                    return Err(Error::NonFinite);
                }
                if d < 0.0 || t <= 0.0 {
                    return Err(Error::Value);
                }
            }
        }
        ConstitutiveKind::LorentzIsotropic => {
            let de = f5.ok_or(Error::Descriptor)?;
            let w0 = f7.ok_or(Error::Descriptor)?;
            let gm = f8.ok_or(Error::Descriptor)?;
            if f6.is_some() || de.element_count() == 0 {
                return Err(Error::Descriptor);
            }
            if de.element_count() != w0.element_count() || de.element_count() != gm.element_count()
            {
                return Err(Error::Shape);
            }
            for i in 0..de.element_count() as usize {
                let d = f64_at(de.value(), i * 8);
                let w = f64_at(w0.value(), i * 8);
                let g = f64_at(gm.value(), i * 8);
                if !d.is_finite() || !w.is_finite() || !g.is_finite() {
                    return Err(Error::NonFinite);
                }
                if w <= 0.0 || g < 0.0 {
                    return Err(Error::Value);
                }
            }
        }
        ConstitutiveKind::AnisotropicTabulated => {
            let att = f9.ok_or(Error::Descriptor)?;
            let entry_id = u32_at(att.value(), 0);
            if entry_id == 0 {
                return Err(Error::Descriptor);
            }
            let _ = ctx.entry(entry_id).map_err(Error::Shared)?;
        }
    }
    if let Some(mat) = field(block, 10) {
        ctx.require_schema(u32_at(mat.value(), 0), 105)
            .map_err(Error::Shared)?;
    }
    if let Some(ev) = field(block, 11) {
        ctx.require_schema(u32_at(ev.value(), 0), 134)
            .map_err(Error::Shared)?;
    }
    Ok(())
}

pub fn validate_model_evidence(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    let _ = non_empty_text(block, 1)?;
    let _ = non_empty_text(block, 2)?;
    let _ = non_empty_text(block, 4)?;
    let _ = EpistemicClass::parse(text(required(block, 8)?)?)?;
    for opt_k in [5, 6, 7, 9, 11] {
        if field(block, opt_k).is_some() {
            let _ = non_empty_text(block, opt_k)?;
        }
    }
    if let Some(clk) = field(block, 12) {
        ctx.require_schema(u32_at(clk.value(), 0), 1)
            .map_err(Error::Shared)?;
    }
    if let Some(ticks) = field(block, 10) {
        if field(block, 12).is_none() {
            return Err(Error::Time);
        }
        if i64_at(ticks.value(), 8) < i64_at(ticks.value(), 0) {
            return Err(Error::Time);
        }
    }
    Ok(())
}

pub fn validate_acquisition(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    ctx.require_schema(scalar32(block, 1)?, 1)
        .map_err(Error::Shared)?;
    let rate_num = scalar64(block, 2)?;
    let rate_den = scalar64(block, 3)?;
    if rate_num == 0 || rate_den == 0 {
        return Err(Error::Time);
    }
    let kind = SignalKind::parse(text(required(block, 4)?)?)?;
    let _ = non_empty_text(block, 5)?;
    ctx.require_schema(scalar32(block, 6)?, 107)
        .map_err(Error::Shared)?;
    if text(required(block, 7)?)? != "I_plus_iQ_exp_minus_iwt" {
        return Err(Error::UnsupportedProfile);
    }
    if let Some(w) = field(block, 8) {
        let _ = WindowKind::parse(text(w)?)?;
    }
    if let Some(fft) = field(block, 9) {
        let n = u32_at(fft.value(), 0);
        if !(1..=4096).contains(&n) {
            return Err(Error::Shape);
        }
    }
    if let Some(hop) = field(block, 10) {
        if u32_at(hop.value(), 0) == 0 {
            return Err(Error::Shape);
        }
    }
    if let Some(norm) = field(block, 11) {
        if text(norm)? != "forward-unscaled-inverse-1/N" {
            return Err(Error::UnsupportedProfile);
        }
    }
    if kind == SignalKind::ComplexStft {
        let _ = field(block, 8).ok_or(Error::Descriptor)?;
        let _ = field(block, 9).ok_or(Error::Descriptor)?;
        let _ = field(block, 10).ok_or(Error::Descriptor)?;
        let _ = field(block, 11).ok_or(Error::Descriptor)?;
    }
    if field(block, 12).is_some() && f64_scalar(block, 12)? < 0.0 {
        return Err(Error::Value);
    }
    if let Some(ch) = field(block, 13) {
        if u32_at(ch.value(), 0) == 0 {
            return Err(Error::Shape);
        }
    }
    Ok(())
}

pub fn validate_bio_context(block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), Error> {
    let _ = non_empty_text(block, 1)?;
    let _ = non_empty_text(block, 2)?;
    let obs = text(required(block, 3)?)?;
    if !matches!(
        obs,
        "current_dipole" | "radiative_observation" | "model_construct"
    ) {
        return Err(Error::Value);
    }
    ctx.require_schema(scalar32(block, 4)?, 4)
        .map_err(Error::Shared)?;
    if let Some(cal) = field(block, 5) {
        ctx.require_schema(u32_at(cal.value(), 0), 107)
            .map_err(Error::Shared)?;
    }
    if let Some(ev) = field(block, 6) {
        ctx.require_schema(u32_at(ev.value(), 0), 134)
            .map_err(Error::Shared)?;
    }
    if let Some(cp) = field(block, 7) {
        let prov = ctx
            .require_schema(u32_at(cp.value(), 0), 4)
            .map_err(Error::Shared)?;
        let receipt = field(prov, 6).ok_or(Error::Descriptor)?;
        if receipt.value().len() != 32 || receipt.value().iter().all(|&b| b == 0) {
            return Err(Error::Descriptor);
        }
    }
    Ok(())
}

/// Resolves schema 132 key 7 (`response`) to a typed Calibration (107) or auxiliary
/// response descriptor (schema 3 -> schema 103) block with explicit units and normalization.
pub struct ProfileResponseValidator<'c, 'v, 'a> {
    pub ctx: &'c Context<'v, 'a>,
}

impl spectrum::ResponseValidator for ProfileResponseValidator<'_, '_, '_> {
    fn validate(&self, id: u32, budget: &mut WorkBudget) -> Result<(), spectrum::Error> {
        budget.charge(16)?;
        let block = self.ctx.block(id).map_err(|_| spectrum::Error::UnsupportedResponse)?;
        match (block.schema_type(), block.revision()) {
            (107, 1) => {
                if validate_calibration(block, self.ctx).is_err() {
                    return Err(spectrum::Error::UnsupportedResponse);
                }
                Ok(())
            }
            (3, 1) => {
                let q = field(block, 7).and_then(|f| std::str::from_utf8(f.value()).ok());
                let u = field(block, 8).and_then(|f| std::str::from_utf8(f.value()).ok());
                if q.is_none_or(str::is_empty) || u.is_none_or(str::is_empty) {
                    return Err(spectrum::Error::UnsupportedResponse);
                }
                let ext_ids = field(block, 18).ok_or(spectrum::Error::UnsupportedResponse)?;
                for chunk in ext_ids.value().chunks_exact(4) {
                    budget.charge(8)?;
                    let ext_id = u32_at(chunk, 0);
                    if let Ok(ext) = self.ctx.require_schema(ext_id, 103) {
                        if let Some(pf) = field(ext, 6) {
                            if let Ok(name) = std::str::from_utf8(pf.value()) {
                                if matches!(name, "spectral_response_v1" | "auxiliary_tensor_v1") {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                Err(spectrum::Error::UnsupportedResponse)
            }
            _ => Err(spectrum::Error::UnsupportedResponse),
        }
    }
}
