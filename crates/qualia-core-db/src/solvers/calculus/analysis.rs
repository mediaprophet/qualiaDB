//! Finite-dimensional analysis foundations used by native calculus.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalysisError {
    NonFinite,
    Degenerate,
    Singular,
    InvalidDomain,
    NotCertified,
    IterationLimit { residual: f64 },
    OutputBufferFull,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector<const N: usize> {
    pub data: [f64; N],
}

impl<const N: usize> Vector<N> {
    pub const fn new(data: [f64; N]) -> Self {
        Self { data }
    }

    pub const fn zero() -> Self {
        Self { data: [0.0; N] }
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        if self.data.iter().all(|value| value.is_finite()) {
            Ok(())
        } else {
            Err(AnalysisError::NonFinite)
        }
    }

    pub fn dot(self, other: Self) -> Result<f64, AnalysisError> {
        self.validate()?;
        other.validate()?;
        Ok(self
            .data
            .iter()
            .zip(other.data)
            .map(|(left, right)| left * right)
            .sum())
    }

    pub fn norm(self) -> Result<f64, AnalysisError> {
        Ok(self.dot(self)?.sqrt())
    }

    pub fn distance(self, other: Self) -> Result<f64, AnalysisError> {
        self.sub(other).norm()
    }

    pub fn add(self, other: Self) -> Self {
        let mut result = [0.0; N];
        for (index, value) in result.iter_mut().enumerate() {
            *value = self.data[index] + other.data[index];
        }
        Self::new(result)
    }

    pub fn sub(self, other: Self) -> Self {
        let mut result = [0.0; N];
        for (index, value) in result.iter_mut().enumerate() {
            *value = self.data[index] - other.data[index];
        }
        Self::new(result)
    }

    pub fn scale(self, scalar: f64) -> Result<Self, AnalysisError> {
        if !scalar.is_finite() {
            return Err(AnalysisError::NonFinite);
        }
        let mut result = self;
        for value in &mut result.data {
            *value *= scalar;
        }
        result.validate()?;
        Ok(result)
    }

    pub fn project_onto(self, direction: Self) -> Result<Self, AnalysisError> {
        let denominator = direction.dot(direction)?;
        if denominator <= f64::MIN_POSITIVE {
            return Err(AnalysisError::Degenerate);
        }
        direction.scale(self.dot(direction)? / denominator)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearMap<const R: usize, const C: usize> {
    pub coefficients: [[f64; C]; R],
}

impl<const R: usize, const C: usize> LinearMap<R, C> {
    pub const fn new(coefficients: [[f64; C]; R]) -> Self {
        Self { coefficients }
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        if self
            .coefficients
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        {
            Ok(())
        } else {
            Err(AnalysisError::NonFinite)
        }
    }

    pub fn apply(&self, input: Vector<C>) -> Result<Vector<R>, AnalysisError> {
        self.validate()?;
        input.validate()?;
        let mut output = [0.0; R];
        for (row, value) in output.iter_mut().enumerate() {
            *value = self.coefficients[row]
                .iter()
                .zip(input.data)
                .map(|(coefficient, component)| coefficient * component)
                .sum();
        }
        Ok(Vector::new(output))
    }

    pub fn transpose(&self) -> LinearMap<C, R> {
        let mut result = [[0.0; R]; C];
        for (row, coefficients) in self.coefficients.iter().enumerate() {
            for (column, coefficient) in coefficients.iter().enumerate() {
                result[column][row] = *coefficient;
            }
        }
        LinearMap::new(result)
    }
}

impl<const N: usize> LinearMap<N, N> {
    pub const fn identity() -> Self {
        let mut coefficients = [[0.0; N]; N];
        let mut index = 0;
        while index < N {
            coefficients[index][index] = 1.0;
            index += 1;
        }
        Self::new(coefficients)
    }

    pub fn trace(&self) -> Result<f64, AnalysisError> {
        self.validate()?;
        Ok((0..N).map(|index| self.coefficients[index][index]).sum())
    }

    pub fn determinant(&self) -> Result<f64, AnalysisError> {
        self.validate()?;
        let mut matrix = self.coefficients;
        let mut determinant = 1.0;
        let mut sign = 1.0;
        for column in 0..N {
            let mut pivot = column;
            for row in column + 1..N {
                if matrix[row][column].abs() > matrix[pivot][column].abs() {
                    pivot = row;
                }
            }
            if matrix[pivot][column].abs() <= f64::EPSILON {
                return Ok(0.0);
            }
            if pivot != column {
                matrix.swap(pivot, column);
                sign = -sign;
            }
            let diagonal = matrix[column][column];
            determinant *= diagonal;
            for row in column + 1..N {
                let factor = matrix[row][column] / diagonal;
                for trailing in column + 1..N {
                    matrix[row][trailing] -= factor * matrix[column][trailing];
                }
            }
        }
        Ok(sign * determinant)
    }

    pub fn solve(&self, rhs: Vector<N>) -> Result<Vector<N>, AnalysisError> {
        self.validate()?;
        rhs.validate()?;
        let mut matrix = self.coefficients;
        let mut values = rhs.data;
        for column in 0..N {
            let mut pivot = column;
            for row in column + 1..N {
                if matrix[row][column].abs() > matrix[pivot][column].abs() {
                    pivot = row;
                }
            }
            if matrix[pivot][column].abs() <= 64.0 * f64::EPSILON {
                return Err(AnalysisError::Singular);
            }
            matrix.swap(pivot, column);
            values.swap(pivot, column);
            for row in column + 1..N {
                let factor = matrix[row][column] / matrix[column][column];
                matrix[row][column] = 0.0;
                for trailing in column + 1..N {
                    matrix[row][trailing] -= factor * matrix[column][trailing];
                }
                values[row] -= factor * values[column];
            }
        }
        let mut solution = [0.0; N];
        for row in (0..N).rev() {
            let mut value = values[row];
            for column in row + 1..N {
                value -= matrix[row][column] * solution[column];
            }
            solution[row] = value / matrix[row][row];
        }
        let result = Vector::new(solution);
        result.validate()?;
        Ok(result)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis<const N: usize> {
    /// Basis vectors are stored as columns.
    pub matrix: LinearMap<N, N>,
}

impl<const N: usize> Basis<N> {
    pub fn new(matrix: LinearMap<N, N>) -> Result<Self, AnalysisError> {
        if matrix.determinant()?.abs() <= 64.0 * f64::EPSILON {
            return Err(AnalysisError::Singular);
        }
        Ok(Self { matrix })
    }

    pub fn from_coordinates(&self, coordinates: Vector<N>) -> Result<Vector<N>, AnalysisError> {
        self.matrix.apply(coordinates)
    }

    pub fn to_coordinates(&self, vector: Vector<N>) -> Result<Vector<N>, AnalysisError> {
        self.matrix.solve(vector)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, AnalysisError> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err(AnalysisError::InvalidDomain);
        }
        Ok(Self { lower, upper })
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.lower && value <= self.upper
    }

    pub fn uniform_cover(
        self,
        radius: f64,
        out_centers: &mut [f64],
    ) -> Result<usize, AnalysisError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(AnalysisError::InvalidDomain);
        }
        let required = (((self.upper - self.lower) / (2.0 * radius)).ceil() as usize).max(1);
        if out_centers.len() < required {
            return Err(AnalysisError::OutputBufferFull);
        }
        let width = (self.upper - self.lower) / required as f64;
        for (index, center) in out_centers[..required].iter_mut().enumerate() {
            *center = self.lower + (index as f64 + 0.5) * width;
        }
        Ok(required)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedPointCertificate<const N: usize> {
    pub point: Vector<N>,
    pub contraction_factor: f64,
    pub iterations: u32,
    pub residual: f64,
    pub a_posteriori_error_bound: f64,
}

pub fn contraction_fixed_point<const N: usize, F>(
    map: F,
    initial: Vector<N>,
    contraction_factor: f64,
    tolerance: f64,
    max_iterations: u32,
) -> Result<FixedPointCertificate<N>, AnalysisError>
where
    F: Fn(Vector<N>) -> Vector<N>,
{
    if !contraction_factor.is_finite()
        || !(0.0..1.0).contains(&contraction_factor)
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err(AnalysisError::NotCertified);
    }
    initial.validate()?;
    let mut current = initial;
    for iteration in 1..=max_iterations {
        let next = map(current);
        next.validate()?;
        let residual = next.distance(current)?;
        let error_bound = contraction_factor * residual / (1.0 - contraction_factor);
        if error_bound <= tolerance {
            return Ok(FixedPointCertificate {
                point: next,
                contraction_factor,
                iterations: iteration,
                residual,
                a_posteriori_error_bound: error_bound,
            });
        }
        current = next;
    }
    let residual = map(current).distance(current)?;
    Err(AnalysisError::IterationLimit { residual })
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn norm(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    pub fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    pub fn exp(self) -> Self {
        let magnitude = self.re.exp();
        Self::new(magnitude * self.im.cos(), magnitude * self.im.sin())
    }

    /// Principal logarithm with argument in `(-pi, pi]`.
    pub fn principal_log(self) -> Result<Self, AnalysisError> {
        let norm = self.norm();
        if !norm.is_finite() || norm == 0.0 {
            return Err(AnalysisError::InvalidDomain);
        }
        Ok(Self::new(norm.ln(), self.im.atan2(self.re)))
    }

    /// Principal square root with non-negative real part.
    pub fn principal_sqrt(self) -> Result<Self, AnalysisError> {
        if !self.re.is_finite() || !self.im.is_finite() {
            return Err(AnalysisError::NonFinite);
        }
        let magnitude = self.norm();
        let re = ((magnitude + self.re) * 0.5).max(0.0).sqrt();
        let im_magnitude = ((magnitude - self.re) * 0.5).max(0.0).sqrt();
        let im = if self.im < 0.0 {
            -im_magnitude
        } else {
            im_magnitude
        };
        Ok(Self::new(re, im))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basis_coordinate_round_trip_and_singular_rejection() {
        let basis = Basis::new(LinearMap::new([[1.0, 1.0], [0.0, 2.0]])).unwrap();
        let coordinates = Vector::new([3.0, -1.0]);
        let vector = basis.from_coordinates(coordinates).unwrap();
        let recovered = basis.to_coordinates(vector).unwrap();
        assert_eq!(recovered, coordinates);
        assert_eq!(
            Basis::new(LinearMap::new([[1.0, 2.0], [2.0, 4.0]])),
            Err(AnalysisError::Singular)
        );
    }

    #[test]
    fn projection_is_idempotent_and_residual_is_orthogonal() {
        let vector = Vector::new([2.0, 3.0, 4.0]);
        let direction = Vector::new([1.0, -1.0, 0.0]);
        let projection = vector.project_onto(direction).unwrap();
        let second = projection.project_onto(direction).unwrap();
        assert!(projection.distance(second).unwrap() < 1e-14);
        assert!(vector.sub(projection).dot(direction).unwrap().abs() < 1e-14);
    }

    #[test]
    fn contraction_certificate_contains_analytic_fixed_point() {
        let certificate = contraction_fixed_point(
            |x: Vector<1>| Vector::new([0.5 * x.data[0] + 1.0]),
            Vector::new([0.0]),
            0.5,
            1e-10,
            128,
        )
        .unwrap();
        assert!((certificate.point.data[0] - 2.0).abs() <= 1e-10);
        assert!(certificate.a_posteriori_error_bound <= 1e-10);
    }

    #[test]
    fn complex_principal_branches_are_explicit_and_consistent() {
        let z = Complex64::new(-3.0, 4.0);
        let root = z.principal_sqrt().unwrap();
        let squared = root.mul(root);
        assert!((squared.re - z.re).abs() < 1e-14);
        assert!((squared.im - z.im).abs() < 1e-14);

        let value = Complex64::new(0.3, -0.7);
        let round_trip = value.exp().principal_log().unwrap();
        assert!((round_trip.re - value.re).abs() < 1e-14);
        assert!((round_trip.im - value.im).abs() < 1e-14);
    }

    #[test]
    fn interval_cover_reports_capacity() {
        let interval = Interval::new(0.0, 1.0).unwrap();
        let mut too_small = [0.0; 2];
        assert_eq!(
            interval.uniform_cover(0.1, &mut too_small),
            Err(AnalysisError::OutputBufferFull)
        );
        let mut centers = [0.0; 5];
        assert_eq!(interval.uniform_cover(0.1, &mut centers), Ok(5));
    }
}
