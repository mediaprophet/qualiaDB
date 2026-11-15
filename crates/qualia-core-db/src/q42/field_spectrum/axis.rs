use super::*;
const fn r(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> FieldRule {
    FieldRule {
        key,
        type_tag,
        count,
        required,
    }
}
const RULES: &[FieldRule] = &[
    r(1, 6, None, true),
    r(2, 6, None, true),
    r(3, 6, None, true),
    r(4, 6, None, true),
    r(5, 5, None, true),
    r(6, 6, None, true),
    r(7, 8, Some(1), false),
    r(8, 6, None, true),
    r(9, 5, Some(2), false),
];
/// A family registry can expose these rules, then call admit_axis with its true N.
pub fn schema_rules() -> &'static [FieldRule] {
    RULES
}
/// Must resolve the exact nonzero metadata ID to a SUPPORTED typed response
/// schema/profile, declared quantity/unit and explicit normalization. Unknown is
/// never a flat response. This is a registry hook, not a default response model.
pub trait ResponseValidator {
    fn validate(&self, id: u32, budget: &mut WorkBudget) -> Result<(), Error>;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct NoResponse;
impl ResponseValidator for NoResponse {
    fn validate(&self, _id: u32, _budget: &mut WorkBudget) -> Result<(), Error> {
        Err(Error::UnsupportedResponse)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SigmaInterpretation {
    None,
    LegacyDisplay400700nm,
    LegacyEtalonDisplay380750nm,
    LogFrequency { minimum_hz: f64, maximum_hz: f64 },
}
/// Borrowed physical coordinates; sigma labels do not replace or infer these data.
#[derive(Clone, Copy, Debug)]
pub struct AxisView<'a> {
    block: Block<'a>,
    coordinates: &'a [u8],
    kind: AxisKind,
    semantics: SampleSemantics,
    quantity: Quantity,
    sigma: SigmaInterpretation,
    response_metadata_id: Option<u32>,
    samples: u64,
    remaining_work: u64,
}
impl<'a> AxisView<'a> {
    pub fn kind(self) -> AxisKind {
        self.kind
    }
    pub fn semantics(self) -> SampleSemantics {
        self.semantics
    }
    pub fn quantity(self) -> Quantity {
        self.quantity
    }
    pub fn sigma(self) -> SigmaInterpretation {
        self.sigma
    }
    pub fn response_metadata_id(self) -> Option<u32> {
        self.response_metadata_id
    }
    pub fn source_block(self) -> Block<'a> {
        self.block
    }
    pub fn sample_count(self) -> u64 {
        self.samples
    }
    pub fn coordinate_count(self) -> u64 {
        (self.coordinates.len() / 8) as u64
    }
    pub fn coordinates(self) -> Coordinates<'a> {
        Coordinates {
            bytes: self.coordinates,
        }
    }
    pub fn coordinate(self, index: u64) -> Result<f64, Error> {
        if index >= self.coordinate_count() {
            return Err(Error::Index);
        }
        Ok(f64_at(self.coordinates, index as usize * 8))
    }
    pub fn remaining_work(self) -> u64 {
        self.remaining_work
    }
    /// Coordinate transforms reverse frequency/wavelength order. This map is
    /// explicit; originals are never reordered, resampled or overwritten.
    pub fn conversion_order(self, target: AxisKind) -> AxisOrder {
        if self.kind == target {
            AxisOrder::Preserved
        } else {
            AxisOrder::Reversed
        }
    }
    pub fn source_coordinate_index_for_ascending(
        self,
        target: AxisKind,
        target_index: u64,
    ) -> Result<u64, Error> {
        self.conversion_order(target)
            .source_index(target_index, self.coordinate_count())
    }
    /// Sample values have N elements even when bin edges have N+1.
    pub fn source_sample_index_for_ascending(
        self,
        target: AxisKind,
        target_index: u64,
    ) -> Result<u64, Error> {
        self.conversion_order(target)
            .source_index(target_index, self.sample_count())
    }
    pub fn ascending_derived_coordinate(self, target: AxisKind, index: u64) -> Result<f64, Error> {
        let source = self.source_coordinate_index_for_ascending(target, index)?;
        let value = self.coordinate(source)?;
        match (self.kind, target) {
            (AxisKind::FrequencyHz, AxisKind::VacuumWavelengthMetres) => {
                frequency_hz_to_vacuum_wavelength_m(value)
            }
            (AxisKind::VacuumWavelengthMetres, AxisKind::FrequencyHz) => {
                vacuum_wavelength_m_to_frequency_hz(value)
            }
            _ => Ok(value),
        }
    }
    /// Legacy display sigma intentionally has no physical frequency conversion.
    pub fn log_frequency_sigma(self, frequency_hz: f64) -> Result<f64, Error> {
        if let SigmaInterpretation::LogFrequency {
            minimum_hz,
            maximum_hz,
        } = self.sigma
        {
            positive(frequency_hz)?;
            if frequency_hz < minimum_hz || frequency_hz > maximum_hz {
                return Err(Error::Sigma);
            }
            if frequency_hz == minimum_hz {
                return Ok(0.0);
            }
            if frequency_hz == maximum_hz {
                return Ok(1.0);
            }
            let value = log_ratio(frequency_hz, minimum_hz) / log_ratio(maximum_hz, minimum_hz);
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(Error::Sigma);
            }
            Ok(value)
        } else {
            Err(Error::Sigma)
        }
    }
}
#[derive(Clone, Debug)]
pub struct Coordinates<'a> {
    bytes: &'a [u8],
}
impl Iterator for Coordinates<'_> {
    type Item = f64;
    fn next(&mut self) -> Option<f64> {
        if self.bytes.is_empty() {
            return None;
        }
        let value = f64_at(self.bytes, 0);
        self.bytes = &self.bytes[8..];
        Some(value)
    }
}
fn log_ratio(high: f64, low: f64) -> f64 {
    let ratio = high / low;
    if ratio <= 2.0 {
        ((high - low) / low).ln_1p()
    } else {
        high.ln() - low.ln()
    }
}
pub fn admit_axis<'a>(
    block: Block<'a>,
    expected_samples: u64,
    limits: Limits,
    response: &impl ResponseValidator,
) -> Result<AxisView<'a>, Error> {
    if block.schema_type() != SCHEMA_ID || block.revision() != 1 {
        return Err(Error::Schema);
    }
    let mut budget = WorkBudget {
        remaining: limits.work,
    };
    budget.charge(block.raw_bytes().len() as u64)?;
    budget.charge(
        (block.fields().count() as u64)
            .checked_mul(32)
            .ok_or(Error::Overflow)?,
    )?;
    validate_fields(block, RULES).map_err(Error::Shared)?;
    if expected_samples == 0 {
        return Err(Error::Count);
    }
    if expected_samples > limits.samples {
        return Err(Error::Limit);
    }
    let kind = match string(required(block, 1)?)? {
        "frequency_hz" => AxisKind::FrequencyHz,
        "vacuum_wavelength_m" => AxisKind::VacuumWavelengthMetres,
        _ => return Err(Error::Axis),
    };
    let semantics = match string(required(block, 2)?)? {
        "point_density" => SampleSemantics::PointDensity,
        "bin_integral" => SampleSemantics::BinIntegral,
        "bin_average" => SampleSemantics::BinAverage,
        _ => return Err(Error::Semantics),
    };
    let quantity_name = string(required(block, 3)?)?;
    let quantity = [
        Quantity::SpectralRadiance,
        Quantity::SpectralIrradiance,
        Quantity::BinIntegratedRadiance,
        Quantity::BinIntegratedIrradiance,
        Quantity::DimensionlessReflectance,
    ]
    .into_iter()
    .find(|q| q.name() == quantity_name)
    .ok_or(Error::QuantityUnit)?;
    if string(required(block, 4)?)? != quantity.unit(kind) {
        return Err(Error::QuantityUnit);
    }
    match (semantics, quantity) {
        (
            SampleSemantics::BinIntegral,
            Quantity::BinIntegratedRadiance | Quantity::BinIntegratedIrradiance,
        ) => {}
        (
            SampleSemantics::PointDensity | SampleSemantics::BinAverage,
            Quantity::SpectralRadiance
            | Quantity::SpectralIrradiance
            | Quantity::DimensionlessReflectance,
        ) => {}
        _ => return Err(Error::QuantityUnit),
    }
    if string(required(block, 6)?)? != "explicit" {
        return Err(Error::Spacing);
    }
    let coordinates = required(block, 5)?;
    let expected = if semantics == SampleSemantics::PointDensity {
        expected_samples
    } else {
        expected_samples.checked_add(1).ok_or(Error::Overflow)?
    };
    if u64::from(coordinates.element_count()) != expected {
        return Err(Error::Count);
    }
    let sigma = match string(required(block, 8)?)? {
        "none" => SigmaInterpretation::None,
        "legacy_display_400_700nm" => SigmaInterpretation::LegacyDisplay400700nm,
        "legacy_etalon_display_380_750nm" => SigmaInterpretation::LegacyEtalonDisplay380750nm,
        "log_frequency_v1" => {
            let f = required(block, 9)?;
            if f.flags() & 1 == 0 {
                return Err(Error::Shared(
                    super::super::field_semantics::Error::RequiredCritical,
                ));
            }
            let lo = f64_at(f.value(), 0);
            let hi = f64_at(f.value(), 8);
            positive(lo)?;
            positive(hi)?;
            if lo >= hi {
                return Err(Error::Sigma);
            }
            SigmaInterpretation::LogFrequency {
                minimum_hz: lo,
                maximum_hz: hi,
            }
        }
        _ => return Err(Error::Sigma),
    };
    if !matches!(sigma, SigmaInterpretation::LogFrequency { .. }) && field(block, 9).is_some() {
        return Err(Error::Sigma);
    }
    let mut prior = 0.0;
    for value in coordinates.value().chunks_exact(8).map(|b| f64_at(b, 0)) {
        budget.charge(1)?;
        positive(value)?;
        if value <= prior {
            return Err(Error::Coordinate);
        }
        prior = value;
        if let SigmaInterpretation::LogFrequency {
            minimum_hz,
            maximum_hz,
        } = sigma
        {
            let f = if kind == AxisKind::FrequencyHz {
                value
            } else {
                vacuum_wavelength_m_to_frequency_hz(value)?
            };
            if f < minimum_hz || f > maximum_hz {
                return Err(Error::Sigma);
            }
        }
    }
    let response_metadata_id = if let Some(f) = field(block, 7) {
        let id = u32::from_le_bytes(f.value().try_into().unwrap());
        if id == 0 {
            return Err(Error::UnsupportedResponse);
        }
        response.validate(id, &mut budget)?;
        Some(id)
    } else {
        None
    };
    Ok(AxisView {
        block,
        coordinates: coordinates.value(),
        kind,
        semantics,
        quantity,
        sigma,
        response_metadata_id,
        samples: expected_samples,
        remaining_work: budget.remaining(),
    })
}
