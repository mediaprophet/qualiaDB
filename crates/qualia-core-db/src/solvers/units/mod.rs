//! **Units, physical constants & dimensional analysis** (Gap analysis §3.6).
//!
//! A unit-correct quantity is a value *plus* its physical dimension, and arithmetic on
//! quantities is **dimensionally checked**: you cannot add a length to a time, and
//! multiplying a force by a distance yields an energy. This serves the NL→3D /
//! engineering work directly (unit-correct geometry and materials) and is a small,
//! self-contained foundation the rest of the engine can lean on.
//!
//! Mission fit: dimensional consistency is a *correctness guard* — a calculation that
//! is dimensionally wrong is wrong, full stop, and the type system catches it before a
//! fabricated number can propagate. Fail-closed throughout ([`UnitsError`]).
//!
//! Layers (one concern per file, §11):
//! * [`dimension`] — the 7-vector of SI base-dimension exponents + named dimensions.
//! * [`quantity`] — a value with a dimension, and checked arithmetic.
//! * [`conversion`] — units (linear factor + affine offset for temperature) and convert.
//! * [`constants`] — CODATA physical constants as dimensioned quantities.
//!
//! Kernel-class `ElementwiseMap` (trivial CPU).

pub mod constants;
pub mod conversion;
pub mod dimension;
pub mod quantity;

pub use conversion::{convert, Unit};
pub use dimension::Dimension;
pub use quantity::Quantity;

/// Fail-closed errors for unit handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsError {
    /// An operation required matching dimensions and they differed (e.g. length + time,
    /// or converting metres to seconds).
    IncompatibleDimensions,
    /// A value could not be represented (e.g. a non-integer dimension exponent).
    InvalidOperation,
}

impl core::fmt::Display for UnitsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UnitsError::IncompatibleDimensions => write!(f, "incompatible physical dimensions"),
            UnitsError::InvalidOperation => write!(f, "invalid dimensional operation"),
        }
    }
}
impl std::error::Error for UnitsError {}
