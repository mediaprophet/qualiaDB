use crate::q42::{field_envelope, field_semantics};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Envelope(field_envelope::Error),
    Shared(field_semantics::Error),
    Unsupported,
    Shape,
    Scalar,
    Unit,
    Clock,
    Overflow,
    Nonfinite,
    Index,
    Limit,
    Profile,
}
impl From<field_envelope::Error> for Error {
    fn from(e: field_envelope::Error) -> Self {
        Self::Envelope(e)
    }
}
impl From<field_semantics::Error> for Error {
    fn from(e: field_semantics::Error) -> Self {
        Self::Shared(e)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Scalar {
    F32 = 1,
    F64 = 2,
    I16 = 3,
    I24 = 4,
    I32 = 5,
    ComplexF32 = 6,
    U8 = 7,
}
impl Scalar {
    pub fn from_code(n: u32) -> Result<Self, Error> {
        Ok(match n {
            1 => Self::F32,
            2 => Self::F64,
            3 => Self::I16,
            4 => Self::I24,
            5 => Self::I32,
            6 => Self::ComplexF32,
            7 => Self::U8,
            _ => return Err(Error::Scalar),
        })
    }
    pub fn bytes(self) -> u32 {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::F64 | Self::ComplexF32 => 8,
            Self::I16 => 2,
            Self::I24 => 3,
            Self::U8 => 1,
        }
    }
}
/// Decoded access retains original bits, never normalizes samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sample {
    F32(f32),
    F64(f64),
    I16(i16),
    I24(i32),
    I32(i32),
    ComplexF32 { real: f32, imag: f32 },
    U8(u8),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    pub numerator: u64,
    pub denominator: u64,
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
impl Rate {
    pub fn new(n: u64, d: u64) -> Result<Self, Error> {
        if n == 0
            || d == 0
            || n > 1_000_000_000_000_000
            || d > 1_000_000_000_000
            || gcd(n.into(), d.into()) != 1
        {
            return Err(Error::Clock);
        }
        Ok(Self {
            numerator: n,
            denominator: d,
        })
    }
    pub fn divided_by(self, n: u32) -> Result<Self, Error> {
        Self::new(self.numerator, self.denominator)?;
        if n == 0 {
            return Err(Error::Clock);
        }
        let g = gcd(self.numerator.into(), n.into()) as u64;
        let num = self.numerator / g;
        let step = u64::from(n) / g;
        let den = self.denominator.checked_mul(step).ok_or(Error::Overflow)?;
        Self::new(num, den)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactTime {
    pub numerator: i128,
    pub denominator: i128,
}
impl ExactTime {
    fn new(n: i128, d: i128) -> Result<Self, Error> {
        if d <= 0 {
            return Err(Error::Clock);
        }
        let g = gcd(n.checked_abs().ok_or(Error::Overflow)? as u128, d as u128) as i128;
        Ok(Self {
            numerator: n / g,
            denominator: d / g,
        })
    }
    fn add(self, n: i128, d: i128) -> Result<Self, Error> {
        let g = gcd(self.denominator as u128, d as u128) as i128;
        let a = d / g;
        let b = self.denominator / g;
        Self::new(
            self.numerator
                .checked_mul(a)
                .and_then(|x| n.checked_mul(b).and_then(|y| x.checked_add(y)))
                .ok_or(Error::Overflow)?,
            self.denominator.checked_mul(a).ok_or(Error::Overflow)?,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub scale: u32,
    pub epoch_seconds: i64,
    pub epoch_nanoseconds: u32,
    pub ticks: Rate,
    pub samples: Rate,
    pub start_tick: i64,
}
impl Timing {
    pub fn time_at(self, index: u64) -> Result<ExactTime, Error> {
        Rate::new(self.ticks.numerator, self.ticks.denominator)?;
        Rate::new(self.samples.numerator, self.samples.denominator)?;
        if self.scale > 2 || self.epoch_nanoseconds >= 1_000_000_000 {
            return Err(Error::Clock);
        }
        ExactTime::new(self.epoch_seconds.into(), 1)?
            .add(self.epoch_nanoseconds.into(), 1_000_000_000)?
            .add(
                i128::from(self.start_tick)
                    .checked_mul(self.ticks.denominator.into())
                    .ok_or(Error::Overflow)?,
                self.ticks.numerator.into(),
            )?
            .add(
                i128::from(index)
                    .checked_mul(self.samples.denominator.into())
                    .ok_or(Error::Overflow)?,
                self.samples.numerator.into(),
            )
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entries: u32,
    pub samples: u64,
    pub payload_bytes: u64,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 4096,
            samples: 16_777_216,
            payload_bytes: 256 << 20,
            work: 64_000_000,
        }
    }
}
