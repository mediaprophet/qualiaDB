//! Common error and status types for computational economics.
//!
//! These are the shared `repr(C)`-friendly enums used across economics
//! kernels. Module-specific errors (e.g. `FixedIncomeError`, `YieldCurveError`)
//! remain in their owning modules when they carry domain-specific failure
//! modes; this module supplies the generic convergence/status vocabulary
//! referenced by the plan's ABI section (§6) and used by DP, Markov, and
//! macro kernels that do not have a more specific error type.

/// Convergence status for iterative economics solvers.
///
/// `repr(u8)` so it is ABI-stable for WASM, edge, and Webizen dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EconStatus {
    /// Solver converged within tolerance before the iteration budget ran out.
    Converged = 0,
    /// Iteration budget exhausted before the residual fell below tolerance.
    MaxIterations = 1,
    /// Inputs failed validation (bad dimensions, non-finite values, etc.).
    InvalidInput = 2,
    /// The linear system or operator is singular / non-invertible.
    Singular = 3,
    /// A caller-supplied output buffer was too small for the request.
    BufferTooSmall = 4,
    /// A non-finite value (NaN/inf) appeared during iteration.
    NonFinite = 5,
}

impl EconStatus {
    /// True when the solver reports a usable result.
    #[inline]
    pub fn is_ok(self) -> bool {
        matches!(self, EconStatus::Converged)
    }

    /// True when the failure is a caller-side buffer/dimension problem.
    #[inline]
    pub fn is_caller_error(self) -> bool {
        matches!(
            self,
            EconStatus::InvalidInput | EconStatus::BufferTooSmall
        )
    }
}

/// Convergence report returned by iterative kernels.
///
/// `repr(C)` so it can cross the WASM / edge / GPU ABI as a fixed record.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct EconConvergence {
    /// Number of iterations actually executed.
    pub iterations: u32,
    /// Final residual norm (definition is kernel-specific; documented at the
    /// call site). `NaN` only when the kernel could not compute a residual.
    pub residual: f64,
    /// Final status of the iterative solve.
    pub status: EconStatus,
}

impl EconConvergence {
    /// Construct a converged report with the given iteration count and
    /// residual.
    pub const fn converged(iterations: u32, residual: f64) -> Self {
        Self {
            iterations,
            residual,
            status: EconStatus::Converged,
        }
    }

    /// Construct a non-converged report with the given status.
    pub const fn stalled(status: EconStatus, iterations: u32, residual: f64) -> Self {
        Self {
            iterations,
            residual,
            status,
        }
    }
}

/// Generic economics kernel error.
///
/// Used by kernels that do not have a more specific error type. Converts to
/// `EconStatus` for ABI-stable reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconError {
    InvalidInput,
    Singular,
    NonConverged,
    BufferTooSmall,
    NonFinite,
    Unsupported,
}

impl EconError {
    /// Map the error to the closest ABI-stable status code.
    pub fn to_status(self) -> EconStatus {
        match self {
            EconError::InvalidInput => EconStatus::InvalidInput,
            EconError::Singular => EconStatus::Singular,
            EconError::NonConverged => EconStatus::MaxIterations,
            EconError::BufferTooSmall => EconStatus::BufferTooSmall,
            EconError::NonFinite => EconStatus::NonFinite,
            EconError::Unsupported => EconStatus::InvalidInput,
        }
    }
}

impl From<EconError> for EconStatus {
    #[inline]
    fn from(err: EconError) -> Self {
        err.to_status()
    }
}

/// A borrowed view over a contiguous economic series.
///
/// `repr(C)` so it can be passed across the WASM / edge ABI. The `stride`
/// field lets callers pass row-major panels without copying.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EconSeriesView<'a> {
    pub values: &'a [f64],
    pub stride: usize,
}

impl<'a> EconSeriesView<'a> {
    /// Construct a unit-stride view over `values`.
    pub const fn new(values: &'a [f64]) -> Self {
        Self { values, stride: 1 }
    }

    /// Number of accessible elements (accounting for stride).
    pub fn len(&self) -> usize {
        if self.stride == 0 {
            0
        } else {
            self.values.len() / self.stride
        }
    }

    /// True when the view is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fetch the element at `index`, or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<f64> {
        if self.stride == 0 {
            return None;
        }
        self.values.get(index * self.stride).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_classification() {
        assert!(EconStatus::Converged.is_ok());
        assert!(!EconStatus::MaxIterations.is_ok());
        assert!(EconStatus::BufferTooSmall.is_caller_error());
        assert!(!EconStatus::Singular.is_caller_error());
    }

    #[test]
    fn convergence_constructors() {
        let ok = EconConvergence::converged(10, 1e-9);
        assert_eq!(ok.status, EconStatus::Converged);
        assert_eq!(ok.iterations, 10);

        let stalled = EconConvergence::stalled(EconStatus::MaxIterations, 100, 0.5);
        assert_eq!(stalled.status, EconStatus::MaxIterations);
        assert_eq!(stalled.iterations, 100);
    }

    #[test]
    fn error_to_status_round_trip() {
        assert_eq!(EconError::Singular.to_status(), EconStatus::Singular);
        assert_eq!(EconError::NonConverged.to_status(), EconStatus::MaxIterations);
        let status: EconStatus = EconError::BufferTooSmall.into();
        assert_eq!(status, EconStatus::BufferTooSmall);
    }

    #[test]
    fn series_view_stride_access() {
        let data = [1.0, 99.0, 2.0, 99.0, 3.0, 99.0];
        let view = EconSeriesView {
            values: &data,
            stride: 2,
        };
        assert_eq!(view.len(), 3);
        assert_eq!(view.get(0), Some(1.0));
        assert_eq!(view.get(1), Some(2.0));
        assert_eq!(view.get(2), Some(3.0));
        assert_eq!(view.get(3), None);
    }

    #[test]
    fn series_view_unit_stride() {
        let data = [10.0, 20.0, 30.0];
        let view = EconSeriesView::new(&data);
        assert_eq!(view.len(), 3);
        assert_eq!(view.get(1), Some(20.0));
    }

    #[test]
    fn zero_stride_is_empty() {
        let data = [1.0, 2.0];
        let view = EconSeriesView {
            values: &data,
            stride: 0,
        };
        assert!(view.is_empty());
        assert_eq!(view.get(0), None);
    }
}
