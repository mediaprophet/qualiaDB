//! Allocation-free QUDT unit normalization for extension arguments.
//! Copyright (c) 2026 Timothy Charles Holborn. CC BY-NC-ND 4.0.
//! Unit identifiers, dimension vectors and multipliers are from QUDT.org 3.5.3
//! (https://github.com/qudt/qudt-public-repo/releases/tag/v3.5.3, CC BY 4.0,
//! https://creativecommons.org/licenses/by/4.0/). The subset and aliases are local.

const PREFIX: &str = "http://qudt.org/vocab/unit/";
const DIMENSION_PREFIX: &str = "http://qudt.org/vocab/dimensionvector/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QudtUnit {
    pub code: &'static str,
    pub iri: &'static str,
    pub dimension_iri: &'static str,
    pub conversion_multiplier_bits: u64,
    aliases: &'static [&'static str],
}

impl QudtUnit {
    pub const fn conversion_multiplier(self) -> f64 {
        f64::from_bits(self.conversion_multiplier_bits)
    }
}

macro_rules! unit {
    ($code:literal, $dimension:literal, $scale:literal, $($alias:literal),+) => {
        QudtUnit {
            code: $code,
            iri: concat!("http://qudt.org/vocab/unit/", $code),
            dimension_iri: concat!("http://qudt.org/vocab/dimensionvector/", $dimension),
            conversion_multiplier_bits: ($scale as f64).to_bits(),
            aliases: &[$($alias),+],
        }
    };
}

pub const QUDT_UNITS: &[QudtUnit] = &[
    unit!("M", "A0E0L1I0M0H0T0D0", 1.0, "Meter", "Metre"),
    unit!("PA", "A0E0L-1I0M1H0T-2D0", 1.0, "Pascal"),
    unit!("HZ", "A0E0L0I0M0H0T-1D0", 1.0, "Hertz"),
    unit!("N-M", "A0E0L2I0M1H0T-2D0", 1.0, "NewtonMeter", "NewtonMetre"),
    unit!("MilliM", "A0E0L1I0M0H0T0D0", 1e-3, "Millimeter", "Millimetre"),
    unit!("NanoM", "A0E0L1I0M0H0T0D0", 1e-9, "Nanometer", "Nanometre"),
    unit!("GRAY", "A0E0L2I0M0H0T-2D0", 1.0, "Gray"),
    unit!("BARN", "A0E0L2I0M0H0T0D0", 1e-28, "Barn"),
    unit!("PA-SEC", "A0E0L-1I0M1H0T-1D0", 1.0, "PascalSecond"),
    unit!("MOL-PER-M3", "A1E0L-3I0M0H0T0D0", 1.0, "MolePerCubicMeter", "MolePerCubicMetre"),
    unit!("CD-PER-M2", "A0E0L-2I1M0H0T0D0", 1.0, "CandelaPerSquareMeter", "CandelaPerSquareMetre"),
    unit!("BYTE", "A0E0L0I0M0H0T0D1", 8.0, "Byte"),
    unit!("UNITLESS", "A0E0L0I0M0H0T0D1", 1.0, "Unitless"),
    unit!("SEC", "A0E0L0I0M0H0T1D0", 1.0, "Second"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QudtUnitError {
    UnknownUnit,
    DimensionMismatch,
    MultiplierMismatch,
}

pub fn normalize_qudt_unit(input: &str) -> Option<&'static QudtUnit> {
    if let Some(alias) = input.strip_prefix("qudt:") {
        QUDT_UNITS.iter().find(|unit| unit.aliases.contains(&alias))
    } else {
        let code = input.strip_prefix("unit:").or_else(|| input.strip_prefix(PREFIX))?;
        QUDT_UNITS.iter().find(|unit| unit.code == code)
    }
}

/// Check a declared dimension and multiplier against the canonical unit.
/// The multiplier is compared as a parsed binary64 value. No arena bytes are consumed.
pub fn validate_qudt_unit(
    input: &str, declared_dimension: &str, declared_multiplier: f64,
) -> Result<&'static QudtUnit, QudtUnitError> {
    let unit = normalize_qudt_unit(input).ok_or(QudtUnitError::UnknownUnit)?;
    let dimension = declared_dimension.strip_prefix("qkdv:")
        .or_else(|| declared_dimension.strip_prefix(DIMENSION_PREFIX));
    if dimension != unit.dimension_iri.strip_prefix(DIMENSION_PREFIX) {
        return Err(QudtUnitError::DimensionMismatch);
    }
    if declared_multiplier.to_bits() != unit.conversion_multiplier_bits {
        return Err(QudtUnitError::MultiplierMismatch);
    }
    Ok(unit)
}
