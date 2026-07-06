//! Numerical and automatic differential calculus.

use super::analysis::{AnalysisError, Complex64, LinearMap, Vector};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivativeEstimate {
    pub value: f64,
    pub absolute_error: f64,
    pub step: f64,
}

pub fn adaptive_central_difference<F>(
    function: F,
    x: f64,
) -> Result<DerivativeEstimate, AnalysisError>
where
    F: Fn(f64) -> f64,
{
    if !x.is_finite() {
        return Err(AnalysisError::NonFinite);
    }
    let h = f64::EPSILON.cbrt() * (1.0 + x.abs());
    let coarse = central_difference(&function, x, h)?;
    let fine = central_difference(&function, x, h * 0.5)?;
    Ok(DerivativeEstimate {
        value: fine + (fine - coarse) / 3.0,
        absolute_error: (fine - coarse).abs() / 3.0,
        step: h * 0.5,
    })
}

fn central_difference<F>(function: &F, x: f64, h: f64) -> Result<f64, AnalysisError>
where
    F: Fn(f64) -> f64,
{
    let high = function(x + h);
    let low = function(x - h);
    if !high.is_finite() || !low.is_finite() {
        return Err(AnalysisError::InvalidDomain);
    }
    Ok((high - low) / (2.0 * h))
}

pub fn complex_step_derivative<F>(function: F, x: f64) -> Result<DerivativeEstimate, AnalysisError>
where
    F: Fn(Complex64) -> Complex64,
{
    if !x.is_finite() {
        return Err(AnalysisError::NonFinite);
    }
    let h = 1e-20;
    let value = function(Complex64::new(x, h));
    if !value.re.is_finite() || !value.im.is_finite() {
        return Err(AnalysisError::InvalidDomain);
    }
    Ok(DerivativeEstimate {
        value: value.im / h,
        absolute_error: h,
        step: h,
    })
}

pub fn jacobian<const M: usize, const N: usize, F>(
    function: F,
    point: Vector<N>,
) -> Result<LinearMap<M, N>, AnalysisError>
where
    F: Fn(Vector<N>) -> Vector<M>,
{
    point.validate()?;
    let mut coefficients = [[0.0; N]; M];
    for column in 0..N {
        let h = f64::EPSILON.cbrt() * (1.0 + point.data[column].abs());
        let mut high = point;
        let mut low = point;
        high.data[column] += h;
        low.data[column] -= h;
        let high_value = function(high);
        let low_value = function(low);
        high_value.validate()?;
        low_value.validate()?;
        for row in 0..M {
            coefficients[row][column] = (high_value.data[row] - low_value.data[row]) / (2.0 * h);
        }
    }
    Ok(LinearMap::new(coefficients))
}

pub fn jvp<const M: usize, const N: usize>(
    derivative: &LinearMap<M, N>,
    direction: Vector<N>,
) -> Result<Vector<M>, AnalysisError> {
    derivative.apply(direction)
}

pub fn vjp<const M: usize, const N: usize>(
    derivative: &LinearMap<M, N>,
    cotangent: Vector<M>,
) -> Result<Vector<N>, AnalysisError> {
    derivative.transpose().apply(cotangent)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dual<const N: usize> {
    pub value: f64,
    pub derivative: [f64; N],
}

impl<const N: usize> Dual<N> {
    pub const fn constant(value: f64) -> Self {
        Self {
            value,
            derivative: [0.0; N],
        }
    }

    pub fn variable(value: f64, index: usize) -> Result<Self, AnalysisError> {
        if index >= N || !value.is_finite() {
            return Err(AnalysisError::InvalidDomain);
        }
        let mut derivative = [0.0; N];
        derivative[index] = 1.0;
        Ok(Self { value, derivative })
    }

    pub fn add(self, other: Self) -> Self {
        let mut derivative = [0.0; N];
        for (index, value) in derivative.iter_mut().enumerate() {
            *value = self.derivative[index] + other.derivative[index];
        }
        Self {
            value: self.value + other.value,
            derivative,
        }
    }

    pub fn mul(self, other: Self) -> Self {
        let mut derivative = [0.0; N];
        for (index, value) in derivative.iter_mut().enumerate() {
            *value = self.derivative[index] * other.value + self.value * other.derivative[index];
        }
        Self {
            value: self.value * other.value,
            derivative,
        }
    }

    pub fn sin(self) -> Self {
        let scale = self.value.cos();
        let mut derivative = self.derivative;
        for value in &mut derivative {
            *value *= scale;
        }
        Self {
            value: self.value.sin(),
            derivative,
        }
    }

    pub fn exp(self) -> Self {
        let value = self.value.exp();
        let mut derivative = self.derivative;
        for component in &mut derivative {
            *component *= value;
        }
        Self { value, derivative }
    }
}

pub fn hessian<const N: usize, F>(
    function: F,
    point: Vector<N>,
) -> Result<LinearMap<N, N>, AnalysisError>
where
    F: Fn(Vector<N>) -> f64,
{
    point.validate()?;
    let mut coefficients = [[0.0; N]; N];
    let base = function(point);
    if !base.is_finite() {
        return Err(AnalysisError::InvalidDomain);
    }
    for row in 0..N {
        let hr = f64::EPSILON.powf(0.25) * (1.0 + point.data[row].abs());
        for column in row..N {
            let hc = f64::EPSILON.powf(0.25) * (1.0 + point.data[column].abs());
            let value = if row == column {
                let mut high = point;
                let mut low = point;
                high.data[row] += hr;
                low.data[row] -= hr;
                (function(high) - 2.0 * base + function(low)) / (hr * hr)
            } else {
                let mut pp = point;
                let mut pm = point;
                let mut mp = point;
                let mut mm = point;
                pp.data[row] += hr;
                pp.data[column] += hc;
                pm.data[row] += hr;
                pm.data[column] -= hc;
                mp.data[row] -= hr;
                mp.data[column] += hc;
                mm.data[row] -= hr;
                mm.data[column] -= hc;
                (function(pp) - function(pm) - function(mp) + function(mm)) / (4.0 * hr * hc)
            };
            if !value.is_finite() {
                return Err(AnalysisError::InvalidDomain);
            }
            coefficients[row][column] = value;
            coefficients[column][row] = value;
        }
    }
    Ok(LinearMap::new(coefficients))
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewtonReport<const N: usize> {
    pub solution: Vector<N>,
    pub iterations: u32,
    pub residual_norm: f64,
    pub step_norm: f64,
}

pub fn damped_newton<const N: usize, F>(
    function: F,
    initial: Vector<N>,
    tolerance: f64,
    max_iterations: u32,
) -> Result<NewtonReport<N>, AnalysisError>
where
    F: Fn(Vector<N>) -> Vector<N>,
{
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(AnalysisError::InvalidDomain);
    }
    initial.validate()?;
    let mut point = initial;
    let mut last_step = 0.0;
    for iteration in 0..=max_iterations {
        let residual = function(point);
        residual.validate()?;
        let residual_norm = residual.norm()?;
        if residual_norm <= tolerance {
            return Ok(NewtonReport {
                solution: point,
                iterations: iteration,
                residual_norm,
                step_norm: last_step,
            });
        }
        if iteration == max_iterations {
            return Err(AnalysisError::IterationLimit {
                residual: residual_norm,
            });
        }
        let derivative = jacobian(&function, point)?;
        let step = derivative.solve(residual.scale(-1.0)?)?;
        last_step = step.norm()?;

        let mut damping = 1.0;
        let mut accepted = false;
        for _ in 0..20 {
            let candidate = point.add(step.scale(damping)?);
            let candidate_norm = function(candidate).norm()?;
            if candidate_norm < residual_norm {
                point = candidate;
                accepted = true;
                break;
            }
            damping *= 0.5;
        }
        if !accepted {
            return Err(AnalysisError::IterationLimit {
                residual: residual_norm,
            });
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_and_complex_step_derivatives_match_analytic_values() {
        let central = adaptive_central_difference(|x| x.sin(), 0.4).unwrap();
        assert!((central.value - 0.4_f64.cos()).abs() < 1e-10);

        let complex = complex_step_derivative(|z| z.exp(), 0.4).unwrap();
        assert!((complex.value - 0.4_f64.exp()).abs() < 1e-14);
    }

    #[test]
    fn jvp_vjp_duality_holds() {
        let derivative = LinearMap::new([[1.0, 2.0], [-3.0, 4.0], [0.5, -2.0]]);
        let direction = Vector::new([2.0, -1.0]);
        let cotangent = Vector::new([3.0, 0.25, -4.0]);
        let left = cotangent.dot(jvp(&derivative, direction).unwrap()).unwrap();
        let right = vjp(&derivative, cotangent).unwrap().dot(direction).unwrap();
        assert!((left - right).abs() < 1e-14);
    }

    #[test]
    fn forward_dual_gradient_matches_analytic_gradient() {
        let x = Dual::<2>::variable(2.0, 0).unwrap();
        let y = Dual::<2>::variable(0.3, 1).unwrap();
        let value = x.mul(x).add(y.sin());
        assert!((value.value - (4.0 + 0.3_f64.sin())).abs() < 1e-14);
        assert!((value.derivative[0] - 4.0).abs() < 1e-14);
        assert!((value.derivative[1] - 0.3_f64.cos()).abs() < 1e-14);
    }

    #[test]
    fn hessian_and_damped_newton_match_quadratic_oracle() {
        let point = Vector::new([1.2, -0.7]);
        let matrix = hessian(
            |x: Vector<2>| {
                3.0 * x.data[0] * x.data[0]
                    + 2.0 * x.data[0] * x.data[1]
                    + 4.0 * x.data[1] * x.data[1]
            },
            point,
        )
        .unwrap();
        assert!((matrix.coefficients[0][0] - 6.0).abs() < 1e-6);
        assert!((matrix.coefficients[0][1] - 2.0).abs() < 1e-6);
        assert!((matrix.coefficients[1][1] - 8.0).abs() < 1e-6);

        let report = damped_newton(
            |x: Vector<2>| Vector::new([x.data[0] * x.data[0] - 2.0, x.data[1] - 3.0]),
            Vector::new([1.0, 0.0]),
            1e-12,
            20,
        )
        .unwrap();
        assert!((report.solution.data[0] - 2.0_f64.sqrt()).abs() < 1e-12);
        assert!((report.solution.data[1] - 3.0).abs() < 1e-12);
    }
}
