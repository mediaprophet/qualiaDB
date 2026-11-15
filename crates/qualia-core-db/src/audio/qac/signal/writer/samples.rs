use super::*;
/// Actual original samples. EncodedLe is borrowed byte-exactly, with no alignment
/// assumption. Native variants serialize finite samples explicitly little endian.
#[derive(Clone, Copy, Debug)]
pub enum PcmSamples<'a> {
    EncodedLe { scalar: Scalar, bytes: &'a [u8] },
    F32(&'a [f32]),
    F64(&'a [f64]),
    I16(&'a [i16]),
    I24(&'a [i32]),
    I32(&'a [i32]),
}
impl PcmSamples<'_> {
    pub fn scalar(self) -> Scalar {
        match self {
            Self::EncodedLe { scalar, .. } => scalar,
            Self::F32(_) => Scalar::F32,
            Self::F64(_) => Scalar::F64,
            Self::I16(_) => Scalar::I16,
            Self::I24(_) => Scalar::I24,
            Self::I32(_) => Scalar::I32,
        }
    }
    pub fn encoded_len(self) -> Result<usize, Error> {
        if matches!(self.scalar(), Scalar::ComplexF32 | Scalar::U8) {
            return Err(Error::Scalar);
        }
        let count = match self {
            Self::EncodedLe { bytes, scalar } => {
                if bytes.len() % scalar.bytes() as usize != 0 {
                    return Err(Error::Shape);
                }
                return Ok(bytes.len());
            }
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::I24(v) => v.len(),
            Self::I32(v) => v.len(),
        };
        count
            .checked_mul(self.scalar().bytes() as usize)
            .ok_or(Error::Overflow)
    }
    pub(crate) fn encode_into(self, out: &mut [u8]) -> Result<usize, Error> {
        let n = self.encoded_len()?;
        if out.len() < n {
            return Err(Error::Envelope(env::Error::Extent));
        }
        match self {
            Self::F32(v) => {
                if v.iter().any(|x| !x.is_finite()) {
                    return Err(Error::Nonfinite);
                }
            }
            Self::F64(v) => {
                if v.iter().any(|x| !x.is_finite()) {
                    return Err(Error::Nonfinite);
                }
            }
            Self::I24(v) => {
                if v.iter().any(|x| !(-8_388_608..=8_388_607).contains(x)) {
                    return Err(Error::Scalar);
                }
            }
            _ => {}
        }
        let out = &mut out[..n];
        macro_rules! le {
            ($v:expr,$n:expr) => {
                for (dst, value) in out.chunks_exact_mut($n).zip($v) {
                    dst.copy_from_slice(&value.to_le_bytes());
                }
            };
        }
        match self {
            Self::EncodedLe { bytes, .. } => out.copy_from_slice(bytes),
            Self::F32(v) => le!(v, 4),
            Self::F64(v) => le!(v, 8),
            Self::I16(v) => le!(v, 2),
            Self::I32(v) => le!(v, 4),
            Self::I24(v) => {
                for (dst, value) in out.chunks_exact_mut(3).zip(v) {
                    dst.copy_from_slice(&value.to_le_bytes()[..3]);
                }
            }
        }
        Ok(n)
    }
}
