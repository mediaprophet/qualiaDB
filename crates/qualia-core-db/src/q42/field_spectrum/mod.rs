//! Candidate physical spectral axes over the full representable positive spectrum.
//! No sensor/manufacturer, frequency-band or colour-display dependency. Schema132
//! references field data; this adds no new .10d/.emf magic or duplicated cell store.
//! Static/aperiodic fields have no carrier axis and must not substitute zero Hz.
use super::field_envelope::{Block, Field};
use super::field_semantics::{field, validate_fields, FieldRule};
mod axis;
mod conversion;
pub use axis::{
    admit_axis, schema_rules, AxisView, Coordinates, NoResponse, ResponseValidator,
    SigmaInterpretation,
};
pub use conversion::*;
pub const SCHEMA_ID: u16 = 132;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Schema,
    Shared(super::field_semantics::Error),
    Limit,
    Overflow,
    Count,
    Axis,
    Semantics,
    QuantityUnit,
    Spacing,
    Coordinate,
    Index,
    Sigma,
    UnsupportedResponse,
    NonPositive,
    NonFinite,
    Unrepresentable,
    NotDensity,
    BinAverageNeedsIntegral,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisKind {
    FrequencyHz,
    VacuumWavelengthMetres,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleSemantics {
    PointDensity,
    BinIntegral,
    BinAverage,
}
/// Exact candidate quantity spellings; aliases are not silently admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    SpectralRadiance,
    SpectralIrradiance,
    BinIntegratedRadiance,
    BinIntegratedIrradiance,
    DimensionlessReflectance,
}
impl Quantity {
    pub fn name(self) -> &'static str {
        match self {
            Self::SpectralRadiance => "spectral_radiance",
            Self::SpectralIrradiance => "spectral_irradiance",
            Self::BinIntegratedRadiance => "bin_integrated_radiance",
            Self::BinIntegratedIrradiance => "bin_integrated_irradiance",
            Self::DimensionlessReflectance => "dimensionless_reflectance",
        }
    }
    pub fn unit(self, axis: AxisKind) -> &'static str {
        match (self, axis) {
            (Self::SpectralRadiance, AxisKind::FrequencyHz) => "W sr^-1 m^-2 Hz^-1",
            (Self::SpectralRadiance, AxisKind::VacuumWavelengthMetres) => "W sr^-1 m^-2 m^-1",
            (Self::SpectralIrradiance, AxisKind::FrequencyHz) => "W m^-2 Hz^-1",
            (Self::SpectralIrradiance, AxisKind::VacuumWavelengthMetres) => "W m^-2 m^-1",
            (Self::BinIntegratedRadiance, _) => "W sr^-1 m^-2",
            (Self::BinIntegratedIrradiance, _) => "W m^-2",
            (Self::DimensionlessReflectance, _) => "1",
        }
    }
    pub fn is_density(self) -> bool {
        matches!(self, Self::SpectralRadiance | Self::SpectralIrradiance)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub samples: u64,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            samples: 65536,
            work: 4_000_000,
        }
    }
}
/// Response providers must charge their own variable work before performing it.
pub struct WorkBudget {
    remaining: u64,
}
impl WorkBudget {
    pub fn charge(&mut self, amount: u64) -> Result<(), Error> {
        self.remaining = self.remaining.checked_sub(amount).ok_or(Error::Limit)?;
        Ok(())
    }
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
}
fn required<'a>(block: Block<'a>, key: u16) -> Result<Field<'a>, Error> {
    field(block, key).ok_or(Error::Shared(super::field_semantics::Error::MissingField))
}
fn string(field: Field<'_>) -> Result<&str, Error> {
    std::str::from_utf8(field.value()).map_err(|_| Error::Coordinate)
}
fn f64_at(bytes: &[u8], at: usize) -> f64 {
    f64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}
#[cfg(test)]
mod tests;
