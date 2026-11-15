use super::*;
use crate::audio::qac::signal::{Error, Rate};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformKind {
    Complex,
    Magnitude,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Rectangular,
    HannSymmetricF32,
}
impl WindowKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rectangular => "rectangular",
            Self::HannSymmetricF32 => "hann-symmetric-authored-f32",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginBasis {
    EpochSampleGrid,
    SourceEntryFirstSample,
}
impl OriginBasis {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EpochSampleGrid => "epoch-sample-grid",
            Self::SourceEntryFirstSample => "source-entry-first-sample",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    pub kind: TransformKind,
    pub fft_size: u32,
    pub hop: u32,
    pub bins: u32,
    pub input_rate: Rate,
    pub origin_samples: i64,
    pub window: WindowKind,
    pub origin_basis: OriginBasis,
}
fn validate_hann_f32(b: Block<'_>, fft_size: u32) -> Result<(), Error> {
    if !fft_size.is_power_of_two() || !(2..=1024).contains(&fft_size) {
        return Err(Error::Shape);
    }
    let wf = sem::field(b, 16).ok_or(Error::Profile)?;
    if wf.flags() & 1 == 0 || wf.type_tag() != 4 || wf.element_count() != fft_size {
        return Err(Error::Profile);
    }
    let bytes = wf.value();
    let n = fft_size as usize;
    if bytes.len() != n * 4 {
        return Err(Error::Shape);
    }
    for i in 0..n {
        let val = f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        let opp = f32::from_le_bytes(bytes[(n - 1 - i) * 4..(n - 1 - i) * 4 + 4].try_into().unwrap());
        if !val.is_finite() || !opp.is_finite() {
            return Err(Error::Nonfinite);
        }
        if (f64::from(val) - f64::from(opp)).abs() > 1e-6 {
            return Err(Error::Profile);
        }
        let expected =
            (0.5 * (1.0 - (std::f64::consts::TAU * (i as f64) / ((n - 1) as f64)).cos())) as f32;
        if (f64::from(val) - f64::from(expected)).abs() > 1e-5 {
            return Err(Error::Profile);
        }
    }
    Ok(())
}
impl Transform {
    pub fn window_kind(self) -> WindowKind {
        self.window
    }
    pub fn origin_basis(self) -> OriginBasis {
        self.origin_basis
    }
    pub fn from_block(b: Block<'_>) -> Result<Self, Error> {
        if b.schema_type() != 201 || b.revision() != 1 {
            return Err(Error::Profile);
        }
        use crate::q42::field_semantics::DomainRegistry;
        sem::validate_fields(b, SignalRegistry.rules(201, 1).ok_or(Error::Profile)?)?;
        let kind = match text(b, 1)? {
            "STFT-complex" => TransformKind::Complex,
            "STFT-magnitude" => TransformKind::Magnitude,
            _ => return Err(Error::Unsupported),
        };
        let fft_size = u32v(b, 2)?;
        let hop = u32v(b, 3)?;
        let bins = u32v(b, 13)?;
        if fft_size == 0 || hop == 0 || bins != fft_size || u32v(b, 7)? != 0 {
            return Err(Error::Shape);
        }
        let window = match text(b, 4)? {
            "rectangular" => {
                if sem::field(b, 16).is_some() {
                    return Err(Error::Unsupported);
                }
                WindowKind::Rectangular
            }
            "hann-symmetric-authored-f32" => {
                validate_hann_f32(b, fft_size)?;
                WindowKind::HannSymmetricF32
            }
            _ => return Err(Error::Unsupported),
        };
        if text(b, 6)? != "forward-unscaled-inverse-1/N"
            || text(b, 9)? != "forward-negative-exponent"
            || text(b, 11)? != "none"
        {
            return Err(Error::Unsupported);
        }
        if sem::field(b, 5).is_some() || sem::field(b, 8).is_some() {
            return Err(Error::Unsupported);
        }
        let origin_samples = u64v(b, 10)? as i64;
        let origin_basis = match sem::field(b, 17) {
            None => OriginBasis::EpochSampleGrid,
            Some(f) => {
                if f.flags() & 1 == 0 {
                    return Err(Error::Profile);
                }
                if text(b, 17)? != "source-entry-first-sample" {
                    return Err(Error::Unsupported);
                }
                if origin_samples != 0 {
                    return Err(Error::Clock);
                }
                OriginBasis::SourceEntryFirstSample
            }
        };
        text(b, 12)?;
        Ok(Self {
            kind,
            fft_size,
            hop,
            bins,
            input_rate: Rate::new(u64v(b, 14)?, u64v(b, 15)?)?,
            origin_samples,
            window,
            origin_basis,
        })
    }
}

/// Signed mathematical FFT offset in Hz, not an inferred physical RF carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrequencyOffset {
    pub numerator: i128,
    pub denominator: i128,
}
impl Transform {
    /// Preserve in-order full bins. For even N, Nyquist is the negative representative.
    pub fn frequency_offset(self, bin: u32) -> Result<FrequencyOffset, Error> {
        Rate::new(self.input_rate.numerator, self.input_rate.denominator)?;
        if self.fft_size == 0 || self.bins != self.fft_size {
            return Err(Error::Shape);
        }
        if bin >= self.bins {
            return Err(Error::Index);
        }
        let signed = if bin <= (self.fft_size - 1) / 2 {
            i128::from(bin)
        } else {
            i128::from(bin) - i128::from(self.fft_size)
        };
        let numerator = signed
            .checked_mul(self.input_rate.numerator.into())
            .ok_or(Error::Overflow)?;
        let denominator = i128::from(self.fft_size)
            .checked_mul(self.input_rate.denominator.into())
            .ok_or(Error::Overflow)?;
        let mut a = numerator.unsigned_abs();
        let mut b = denominator as u128;
        while b != 0 {
            let r = a % b;
            a = b;
            b = r;
        }
        let gcd = a as i128;
        Ok(FrequencyOffset {
            numerator: numerator / gcd,
            denominator: denominator / gcd,
        })
    }
}
