//! Finite-dimensional chart and Riemannian metric primitives.

use super::analysis::{AnalysisError, Interval, LinearMap, Vector};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chart<const N: usize> {
    pub chart_id: u64,
    pub coordinate_domain: [Interval; N],
}

impl<const N: usize> Chart<N> {
    pub fn contains(&self, point: Vector<N>) -> bool {
        self.coordinate_domain
            .iter()
            .zip(point.data)
            .all(|(interval, value)| interval.contains(value))
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionEvidence {
    pub samples_checked: usize,
    pub maximum_round_trip_error: f64,
    pub tolerance: f64,
}

pub fn verify_transition<const N: usize, F, G>(
    source: &Chart<N>,
    target: &Chart<N>,
    samples: &[Vector<N>],
    forward: F,
    inverse: G,
    tolerance: f64,
) -> Result<TransitionEvidence, AnalysisError>
where
    F: Fn(Vector<N>) -> Vector<N>,
    G: Fn(Vector<N>) -> Vector<N>,
{
    if samples.is_empty() || !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(AnalysisError::InvalidDomain);
    }
    let mut maximum_error = 0.0_f64;
    for sample in samples {
        if !source.contains(*sample) {
            return Err(AnalysisError::InvalidDomain);
        }
        let mapped = forward(*sample);
        mapped.validate()?;
        if !target.contains(mapped) {
            return Err(AnalysisError::InvalidDomain);
        }
        let recovered = inverse(mapped);
        let error = recovered.distance(*sample)?;
        maximum_error = maximum_error.max(error);
    }
    if maximum_error > tolerance {
        return Err(AnalysisError::NotCertified);
    }
    Ok(TransitionEvidence {
        samples_checked: samples.len(),
        maximum_round_trip_error: maximum_error,
        tolerance,
    })
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiemannMetric<const N: usize> {
    pub coefficients: LinearMap<N, N>,
}

impl<const N: usize> RiemannMetric<N> {
    pub fn new(coefficients: LinearMap<N, N>) -> Result<Self, AnalysisError> {
        coefficients.validate()?;
        for row in 0..N {
            for column in 0..N {
                let scale = coefficients.coefficients[row][column]
                    .abs()
                    .max(coefficients.coefficients[column][row].abs())
                    .max(1.0);
                if (coefficients.coefficients[row][column] - coefficients.coefficients[column][row])
                    .abs()
                    > 32.0 * f64::EPSILON * scale
                {
                    return Err(AnalysisError::InvalidDomain);
                }
            }
        }
        // Sylvester's criterion via leading principal determinants.
        for order in 1..=N {
            let mut block = [[0.0; N]; N];
            for row in 0..order {
                for column in 0..order {
                    block[row][column] = coefficients.coefficients[row][column];
                }
            }
            // Fill unused diagonal entries so the full determinant equals the
            // leading-principal determinant.
            for index in order..N {
                block[index][index] = 1.0;
            }
            if LinearMap::new(block).determinant()? <= 0.0 {
                return Err(AnalysisError::InvalidDomain);
            }
        }
        Ok(Self { coefficients })
    }

    pub fn lower(&self, tangent: Vector<N>) -> Result<Vector<N>, AnalysisError> {
        self.coefficients.apply(tangent)
    }

    pub fn raise(&self, covector: Vector<N>) -> Result<Vector<N>, AnalysisError> {
        self.coefficients.solve(covector)
    }

    pub fn inner(&self, left: Vector<N>, right: Vector<N>) -> Result<f64, AnalysisError> {
        left.dot(self.lower(right)?)
    }

    pub fn volume_density(&self) -> Result<f64, AnalysisError> {
        Ok(self.coefficients.determinant()?.sqrt())
    }
}

pub fn orientation_sign<const N: usize>(
    transition_derivative: &LinearMap<N, N>,
) -> Result<i8, AnalysisError> {
    let determinant = transition_derivative.determinant()?;
    if determinant > 0.0 {
        Ok(1)
    } else if determinant < 0.0 {
        Ok(-1)
    } else {
        Err(AnalysisError::Singular)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_transition_round_trip_is_certified() {
        let domain = [
            Interval::new(-10.0, 10.0).unwrap(),
            Interval::new(-10.0, 10.0).unwrap(),
        ];
        let source = Chart {
            chart_id: 1,
            coordinate_domain: domain,
        };
        let target = Chart {
            chart_id: 2,
            coordinate_domain: domain,
        };
        let samples = [
            Vector::new([0.0, 0.0]),
            Vector::new([1.0, -2.0]),
            Vector::new([-3.0, 4.0]),
        ];
        let evidence = verify_transition(
            &source,
            &target,
            &samples,
            |x| Vector::new([x.data[0] + x.data[1], x.data[0] - x.data[1]]),
            |y| Vector::new([0.5 * (y.data[0] + y.data[1]), 0.5 * (y.data[0] - y.data[1])]),
            1e-14,
        )
        .unwrap();
        assert_eq!(evidence.samples_checked, 3);
    }

    #[test]
    fn musical_maps_and_volume_density_are_consistent() {
        let metric = RiemannMetric::new(LinearMap::new([[4.0, 1.0], [1.0, 3.0]])).unwrap();
        let vector = Vector::new([2.0, -1.0]);
        let covector = metric.lower(vector).unwrap();
        let recovered = metric.raise(covector).unwrap();
        assert!(recovered.distance(vector).unwrap() < 1e-14);
        assert!((metric.volume_density().unwrap() - 11.0_f64.sqrt()).abs() < 1e-14);
        assert_eq!(
            orientation_sign(&LinearMap::new([[0.0, 1.0], [1.0, 0.0]])),
            Ok(-1)
        );
    }
}
