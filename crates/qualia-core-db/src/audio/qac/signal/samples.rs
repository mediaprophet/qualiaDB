//! Immutable admitted sample access; returned metadata copies cannot mutate the view.
use super::*;
impl<'a> SignalView<'a> {
    pub fn as_bytes(self) -> &'a [u8] {
        self.payload
    }
    pub fn time_at(self, index: u64) -> Result<ExactTime, Error> {
        if index >= self.records {
            return Err(Error::Index);
        }
        self.timing.ok_or(Error::Clock)?.time_at(index)
    }
    pub fn sample(self, record: u64, channel: u32, bin: u32) -> Result<Sample, Error> {
        if record >= self.records || channel >= self.channels || bin >= self.bins {
            return Err(Error::Index);
        }
        let i = record
            .checked_mul(self.channels.into())
            .and_then(|x| x.checked_add(channel.into()))
            .and_then(|x| x.checked_mul(self.bins.into()))
            .and_then(|x| x.checked_add(bin.into()))
            .ok_or(Error::Overflow)?;
        let start = usize::try_from(
            i.checked_mul(self.scalar.bytes().into())
                .ok_or(Error::Overflow)?,
        )
        .map_err(|_| Error::Overflow)?;
        let n = self.scalar.bytes() as usize;
        decode_sample(
            self.scalar,
            self.payload
                .get(start..start.checked_add(n).ok_or(Error::Overflow)?)
                .ok_or(Error::Index)?,
        )
    }
}
/// Decode exactly one finite original scalar without normalization or calibration.
/// This validates scalar representation only; it does not admit a signal/profile.
pub fn decode_sample(s: Scalar, b: &[u8]) -> Result<Sample, Error> {
    if b.len() != s.bytes() as usize {
        return Err(Error::Scalar);
    }
    finite(s, b)?;
    Ok(match s {
        Scalar::F32 => Sample::F32(f32::from_le_bytes(b.try_into().map_err(|_| Error::Scalar)?)),
        Scalar::F64 => Sample::F64(f64::from_le_bytes(b.try_into().map_err(|_| Error::Scalar)?)),
        Scalar::I16 => Sample::I16(i16::from_le_bytes(b.try_into().map_err(|_| Error::Scalar)?)),
        Scalar::I24 => {
            if b.len() != 3 {
                return Err(Error::Scalar);
            }
            let x = i32::from_le_bytes([b[0], b[1], b[2], if b[2] & 128 != 0 { 255 } else { 0 }]);
            Sample::I24(x)
        }
        Scalar::I32 => Sample::I32(i32::from_le_bytes(b.try_into().map_err(|_| Error::Scalar)?)),
        Scalar::ComplexF32 => {
            if b.len() != 8 {
                return Err(Error::Scalar);
            }
            Sample::ComplexF32 {
                real: f32::from_le_bytes(b[..4].try_into().unwrap()),
                imag: f32::from_le_bytes(b[4..].try_into().unwrap()),
            }
        }
        Scalar::U8 => Sample::U8(*b.first().ok_or(Error::Scalar)?),
    })
}
pub(super) fn finite(s: Scalar, payload: &[u8]) -> Result<(), Error> {
    let width = match s {
        Scalar::F32 | Scalar::ComplexF32 => 4,
        Scalar::F64 => 8,
        _ => return Ok(()),
    };
    for raw in payload.chunks_exact(width) {
        let valid = if width == 4 {
            f32::from_le_bytes(raw.try_into().unwrap()).is_finite()
        } else {
            f64::from_le_bytes(raw.try_into().unwrap()).is_finite()
        };
        if !valid {
            return Err(Error::Nonfinite);
        }
    }
    Ok(())
}

impl<'a> SignalView<'a> {
    pub fn mode(self) -> u16 {
        self.mode
    }
    pub fn scalar(self) -> Scalar {
        self.scalar
    }
    pub fn channels(self) -> u32 {
        self.channels
    }
    pub fn bins(self) -> u32 {
        self.bins
    }
    pub fn records(self) -> u64 {
        self.records
    }
    pub fn stream_id(self) -> u32 {
        self.stream_id
    }
    pub fn frame_id(self) -> u32 {
        self.frame_id
    }
    pub fn timing(self) -> Option<Timing> {
        self.timing
    }
    pub fn transform(self) -> Option<crate::audio::qac::metadata::Transform> {
        self.transform
    }
    pub fn window_coefficients_le(&self) -> Option<&'a [u8]> {
        self.window_coefficients_le
    }
    pub fn channel_order(self) -> &'a str {
        self.channel_order
    }
    pub fn purpose(self) -> &'a str {
        self.purpose
    }
    pub fn relation(self) -> &'a str {
        self.relation
    }
    pub fn quantity(self) -> &'a str {
        self.quantity
    }
    pub fn units(self) -> &'a str {
        self.units
    }
    pub fn units_per_stored_sample(self) -> f64 {
        self.units_per_stored_sample
    }
    pub fn stored_offset(self) -> f64 {
        self.stored_offset
    }
}
