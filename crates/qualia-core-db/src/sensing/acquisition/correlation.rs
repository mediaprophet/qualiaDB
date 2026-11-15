use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductConvention {
    LeftTimesConjugateRight,
    ConjugateLeftTimesRight,
    LeftTimesRight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normalization {
    Sum,
    Mean { contributing_pairs: u64 },
    Reported(ArtifactLink),
}
/// Explicit R_ab(lag): left/right order, lag sign, product and normalization retained.
/// No positivity/Hermitian/quantum-state validity or estimator is inferred.
#[derive(Clone, Copy, Debug)]
pub struct CorrelationView<'a> {
    tensor: TensorView<'a>,
    clock: Clock,
    first_lag: i64,
    lag_step: i64,
    product: ProductConvention,
    normalization: Normalization,
}
impl<'a> CorrelationView<'a> {
    pub fn admit(
        tensor: TensorView<'a>,
        clock: Clock,
        first_lag: i64,
        lag_step: i64,
        product: ProductConvention,
        normalization: Normalization,
    ) -> Result<Self, Error> {
        let l = tensor.layout();
        if l.rank != 3
            || l.axes[..3] != [Axis::Lag, Axis::LeftChannel, Axis::RightChannel]
            || !l.encoding.complex()
            || lag_step == 0
            || tensor.clock().is_some()
        {
            return Err(Error::Shape);
        }
        match normalization {
            Normalization::Mean {
                contributing_pairs: 0,
            } => return Err(Error::Semantics),
            Normalization::Reported(r) => r.validate()?,
            _ => (),
        }
        let view = Self {
            tensor,
            clock,
            first_lag,
            lag_step,
            product,
            normalization,
        };
        view.lag_ticks(l.extents[0] - 1)?;
        Ok(view)
    }
    /// Positive lag means the right observation is later than the left by this duration.
    pub fn lag_ticks(self, index: u32) -> Result<i64, Error> {
        if index >= self.tensor.layout().extents[0] {
            return Err(Error::Bounds);
        }
        self.lag_step
            .checked_mul(i64::from(index))
            .and_then(|x| self.first_lag.checked_add(x))
            .ok_or(Error::Overflow)
    }
    /// Exact lag duration (not an epoch timestamp).
    pub fn lag_seconds(self, index: u32) -> Result<ExactTime, Error> {
        let ticks = self.lag_ticks(index)?;
        let r = self.clock.timing().ticks;
        let n = i128::from(ticks)
            .checked_mul(i128::from(r.denominator))
            .ok_or(Error::Overflow)?;
        let mut a = n.unsigned_abs();
        let mut b = u128::from(r.numerator);
        while b != 0 {
            let c = a % b;
            a = b;
            b = c;
        }
        Ok(ExactTime {
            numerator: n / (a as i128),
            denominator: i128::from(r.numerator) / (a as i128),
        })
    }
    pub fn tensor(self) -> TensorView<'a> {
        self.tensor
    }
    pub fn clock(self) -> Clock {
        self.clock
    }
    pub fn lag_step_ticks(self) -> i64 {
        self.lag_step
    }
    pub fn product(self) -> ProductConvention {
        self.product
    }
    pub fn normalization(self) -> Normalization {
        self.normalization
    }
}
