use super::*;
/// Exact SI defining constants, represented in f64 (normal rounding applies).
pub const VACUUM_SPEED_OF_LIGHT_M_PER_S: f64 = 299_792_458.0;
pub const PLANCK_JOULE_SECOND: f64 = 6.626_070_15e-34;
pub const ELECTRON_VOLT_JOULE: f64 = 1.602_176_634e-19;
const HC: f64 = PLANCK_JOULE_SECOND * VACUUM_SPEED_OF_LIGHT_M_PER_S;
pub(super) fn positive(value: f64) -> Result<(), Error> {
    if !value.is_finite() {
        return Err(Error::NonFinite);
    }
    if value <= 0.0 {
        return Err(Error::NonPositive);
    }
    Ok(())
}
fn checked_positive(value: f64) -> Result<f64, Error> {
    if !value.is_finite() || value <= 0.0 {
        Err(Error::Unrepresentable)
    } else {
        Ok(value)
    }
}
fn checked_density(value: f64) -> Result<(), Error> {
    if !value.is_finite() {
        return Err(Error::NonFinite);
    }
    if value < 0.0 {
        return Err(Error::NonPositive);
    }
    Ok(())
}
/// Positive carrier only; static/aperiodic fields must retain absence instead.
pub fn frequency_hz_to_vacuum_wavelength_m(frequency_hz: f64) -> Result<f64, Error> {
    positive(frequency_hz)?;
    checked_positive(VACUUM_SPEED_OF_LIGHT_M_PER_S / frequency_hz)
}
pub fn vacuum_wavelength_m_to_frequency_hz(wavelength_m: f64) -> Result<f64, Error> {
    positive(wavelength_m)?;
    checked_positive(VACUUM_SPEED_OF_LIGHT_M_PER_S / wavelength_m)
}
pub fn frequency_hz_to_photon_energy_j(frequency_hz: f64) -> Result<f64, Error> {
    positive(frequency_hz)?;
    checked_positive(PLANCK_JOULE_SECOND * frequency_hz)
}
pub fn frequency_hz_to_photon_energy_ev(frequency_hz: f64) -> Result<f64, Error> {
    positive(frequency_hz)?;
    checked_positive((PLANCK_JOULE_SECOND / ELECTRON_VOLT_JOULE) * frequency_hz)
}
pub fn vacuum_wavelength_m_to_photon_energy_j(wavelength_m: f64) -> Result<f64, Error> {
    positive(wavelength_m)?;
    checked_positive(HC / wavelength_m)
}
pub fn photon_energy_j_to_frequency_hz(energy_j: f64) -> Result<f64, Error> {
    positive(energy_j)?;
    checked_positive(energy_j / PLANCK_JOULE_SECOND)
}
pub fn photon_energy_j_to_vacuum_wavelength_m(energy_j: f64) -> Result<f64, Error> {
    positive(energy_j)?;
    checked_positive(HC / energy_j)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisOrder {
    Preserved,
    Reversed,
}
impl AxisOrder {
    /// Explicit permutation for derived ascending coordinates AND matching data.
    pub fn source_index(self, target_index: u64, count: u64) -> Result<u64, Error> {
        if target_index >= count {
            return Err(Error::Index);
        }
        Ok(match self {
            Self::Preserved => target_index,
            Self::Reversed => count - 1 - target_index,
        })
    }
}
/// Nanometres are explicit DERIVED density units; schema132 coordinates stay SI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DensityCoordinate {
    Hertz,
    VacuumMetre,
    VacuumNanometre,
}
fn coordinate(
    value: f64,
    source: DensityCoordinate,
    target: DensityCoordinate,
) -> Result<f64, Error> {
    positive(value)?;
    use DensityCoordinate::*;
    match (source, target) {
        (a, b) if a == b => Ok(value),
        (Hertz, VacuumMetre) => frequency_hz_to_vacuum_wavelength_m(value),
        (Hertz, VacuumNanometre) => {
            checked_positive(frequency_hz_to_vacuum_wavelength_m(value)? * 1e9)
        }
        (VacuumMetre, Hertz) => vacuum_wavelength_m_to_frequency_hz(value),
        (VacuumNanometre, Hertz) => checked_positive((VACUUM_SPEED_OF_LIGHT_M_PER_S * 1e9) / value),
        (VacuumMetre, VacuumNanometre) => checked_positive(value * 1e9),
        (VacuumNanometre, VacuumMetre) => checked_positive(value * 1e-9),
        _ => unreachable!(),
    }
}
fn order(source: DensityCoordinate, target: DensityCoordinate) -> AxisOrder {
    if (source == DensityCoordinate::Hertz) == (target == DensityCoordinate::Hertz) {
        AxisOrder::Preserved
    } else {
        AxisOrder::Reversed
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DensityJacobian {
    pub factor: f64,
    pub order: AxisOrder,
}
impl DensityJacobian {
    /// Unsigned physical radiance/irradiance only. No clipping or normalization.
    pub fn apply(self, density: f64) -> Result<f64, Error> {
        positive(self.factor)?;
        scale(density, self.factor)
    }
}
/// |d(source coordinate)/d(target coordinate)|, using source units as declared.
/// This helper is exclusively for point densities. Bin averages must preserve
/// their integral and divide by the NEW bin width, not reuse a point derivative.
/// Reflectance and bin integrals are not densities and reject this operation.
pub fn density_coordinate_jacobian(
    quantity: Quantity,
    semantics: SampleSemantics,
    source: DensityCoordinate,
    target: DensityCoordinate,
    at_source: f64,
) -> Result<DensityJacobian, Error> {
    if !quantity.is_density() || semantics == SampleSemantics::BinIntegral {
        return Err(Error::NotDensity);
    }
    if semantics == SampleSemantics::BinAverage {
        return Err(Error::BinAverageNeedsIntegral);
    }
    let target_value = coordinate(at_source, source, target)?;
    let factor = if source == target {
        1.0
    } else {
        checked_positive(at_source / target_value)?
    };
    Ok(DensityJacobian {
        factor,
        order: order(source, target),
    })
}
fn scale(density: f64, factor: f64) -> Result<f64, Error> {
    checked_density(density)?;
    let result = density * factor;
    if !result.is_finite() || (density != 0.0 && result == 0.0) {
        return Err(Error::Unrepresentable);
    }
    Ok(result)
}
/// Explicit per-metre to per-nanometre derived values (factor1e-9).
/// Linear unit scaling also applies to density bin averages, never to integrals.
pub fn density_per_metre_to_per_nanometre(
    quantity: Quantity,
    semantics: SampleSemantics,
    value: f64,
) -> Result<f64, Error> {
    if !quantity.is_density() || semantics == SampleSemantics::BinIntegral {
        return Err(Error::NotDensity);
    }
    scale(value, 1e-9)
}
pub fn density_per_nanometre_to_per_metre(
    quantity: Quantity,
    semantics: SampleSemantics,
    value: f64,
) -> Result<f64, Error> {
    if !quantity.is_density() || semantics == SampleSemantics::BinIntegral {
        return Err(Error::NotDensity);
    }
    scale(value, 1e9)
}
/// A derived ascending bin with explicit source-to-target order. Original edges
/// and source values remain untouched. Integral units retain radiance/irradiance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvertedBin {
    pub lower: f64,
    pub upper: f64,
    pub average: f64,
    pub integral: f64,
    pub order: AxisOrder,
}
pub fn convert_bin_average(
    quantity: Quantity,
    source: DensityCoordinate,
    target: DensityCoordinate,
    lower: f64,
    upper: f64,
    average: f64,
) -> Result<ConvertedBin, Error> {
    if !quantity.is_density() {
        return Err(Error::NotDensity);
    }
    positive(lower)?;
    positive(upper)?;
    if lower >= upper {
        return Err(Error::Coordinate);
    }
    let width = checked_positive(upper - lower)?;
    let integral = scale(average, width)?;
    let first = coordinate(lower, source, target)?;
    let last = coordinate(upper, source, target)?;
    let (lower, upper) = if first < last {
        (first, last)
    } else {
        (last, first)
    };
    let target_width = checked_positive(upper - lower)?;
    let average = if integral == 0.0 {
        integral
    } else {
        checked_positive(integral / target_width)?
    };
    Ok(ConvertedBin {
        lower,
        upper,
        average,
        integral,
        order: order(source, target),
    })
}
