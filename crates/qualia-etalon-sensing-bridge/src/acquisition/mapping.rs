use etalon_sense_core::{
    buffer::SampleLayout,
    observation::{ObservationEnvelope, ObservationTimeAnchor},
    quantity::SampleEncoding,
};
use qualia_core_db::sensing::acquisition as a;
#[derive(Clone, Copy, Debug)]
pub struct AcquisitionMapping {
    pub axes: [a::Axis; 4],
    pub byte_order: a::ByteOrder,
    pub components: a::ComponentOrder,
    pub context: a::Context,
    /// Only an exact same-domain FirstSample mapping is supported here.
    /// Unknown/native non-sample timestamps remain in the unchanged envelope.
    pub clock: Option<a::Clock>,
    pub signal_axes: a::SignalAxes,
}
/// Translate the actual Etalon layout without reshaping, copying or unit inference.
pub fn map_layout(
    layout: &SampleLayout,
    mapping: AcquisitionMapping,
) -> Result<a::Layout, a::Error> {
    let encoding = match *layout.encoding() {
        SampleEncoding::U8 => a::Encoding::Shared(a::Scalar::U8),
        SampleEncoding::U16 => a::Encoding::U16,
        SampleEncoding::I16 => a::Encoding::Shared(a::Scalar::I16),
        SampleEncoding::I32 => a::Encoding::Shared(a::Scalar::I32),
        SampleEncoding::F32 => a::Encoding::Shared(a::Scalar::F32),
        SampleEncoding::F64 => a::Encoding::Shared(a::Scalar::F64),
        SampleEncoding::ComplexI16 => a::Encoding::ComplexI16,
        SampleEncoding::ComplexF32 => a::Encoding::Shared(a::Scalar::ComplexF32),
    };
    let mut strides = [0usize; 4];
    for (to, from) in strides.iter_mut().zip(layout.strides_bytes()) {
        *to = usize::try_from(*from).map_err(|_| a::Error::Overflow)?;
    }
    let result = a::Layout {
        rank: layout.rank(),
        extents: *layout.extents(),
        strides,
        axes: mapping.axes,
        encoding,
        byte_order: mapping.byte_order,
        components: mapping.components,
    };
    let span = usize::try_from(layout.required_byte_span().map_err(|_| a::Error::Shape)?)
        .map_err(|_| a::Error::Overflow)?;
    if result.span()? != span {
        return Err(a::Error::Shape);
    }
    Ok(result)
}
pub(super) fn validate_clock(
    observation: ObservationEnvelope,
    clock: Option<a::Clock>,
) -> Result<(), a::Error> {
    let Some(clock) = clock else {
        return Ok(());
    };
    let stamp = observation.timestamp().ok_or(a::Error::Clock)?;
    if observation.time_anchor() != ObservationTimeAnchor::FirstSample
        || clock.origin().value() != Some(stamp.clock_id())
        || clock.restart() != u64::from(stamp.restart_epoch())
        || clock.timing().scale != 0
    {
        return Err(a::Error::Clock);
    }
    let mut n = stamp.ticks_ns();
    let mut d = 1_000_000_000u64;
    let (mut x, mut y) = (n, d);
    while y != 0 {
        let r = x % y;
        x = y;
        y = r;
    }
    n /= x;
    d /= x;
    if clock.time_at(0)?
        != (a::ExactTime {
            numerator: n.into(),
            denominator: d.into(),
        })
    {
        return Err(a::Error::Clock);
    }
    Ok(())
}

#[cfg(test)]
#[path = "mapping_tests.rs"]
mod tests;
