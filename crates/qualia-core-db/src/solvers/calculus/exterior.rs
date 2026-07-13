//! Bounded exterior algebra for three-dimensional calculus and geometry.

use super::analysis::{AnalysisError, LinearMap, Vector};

pub const EXTERIOR3_COMPONENTS: usize = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exterior3 {
    /// Coefficients indexed by basis bit mask: 0, e1, e2, e12, e3, ...
    pub coefficients: [f64; EXTERIOR3_COMPONENTS],
}

impl Exterior3 {
    pub const fn zero() -> Self {
        Self {
            coefficients: [0.0; EXTERIOR3_COMPONENTS],
        }
    }

    pub const fn scalar(value: f64) -> Self {
        let mut coefficients = [0.0; EXTERIOR3_COMPONENTS];
        coefficients[0] = value;
        Self { coefficients }
    }

    pub const fn basis(mask: usize, value: f64) -> Self {
        let mut coefficients = [0.0; EXTERIOR3_COMPONENTS];
        if mask < EXTERIOR3_COMPONENTS {
            coefficients[mask] = value;
        }
        Self { coefficients }
    }

    pub fn validate(&self) -> Result<(), AnalysisError> {
        if self.coefficients.iter().all(|value| value.is_finite()) {
            Ok(())
        } else {
            Err(AnalysisError::NonFinite)
        }
    }

    pub fn grade(self, grade: u32) -> Self {
        let mut result = Self::zero();
        for mask in 0..EXTERIOR3_COMPONENTS {
            if mask.count_ones() == grade {
                result.coefficients[mask] = self.coefficients[mask];
            }
        }
        result
    }

    pub fn add(self, other: Self) -> Self {
        let mut result = Self::zero();
        for mask in 0..EXTERIOR3_COMPONENTS {
            result.coefficients[mask] = self.coefficients[mask] + other.coefficients[mask];
        }
        result
    }

    pub fn scale(self, scalar: f64) -> Result<Self, AnalysisError> {
        if !scalar.is_finite() {
            return Err(AnalysisError::NonFinite);
        }
        let mut result = self;
        for value in &mut result.coefficients {
            *value *= scalar;
        }
        Ok(result)
    }

    pub fn wedge(self, other: Self) -> Result<Self, AnalysisError> {
        self.validate()?;
        other.validate()?;
        let mut result = Self::zero();
        for left in 0..EXTERIOR3_COMPONENTS {
            for right in 0..EXTERIOR3_COMPONENTS {
                if left & right != 0 {
                    continue;
                }
                let (mask, sign) = wedge_basis(left, right);
                result.coefficients[mask] +=
                    sign * self.coefficients[left] * other.coefficients[right];
            }
        }
        Ok(result)
    }

    pub fn interior(self, vector: Vector<3>) -> Result<Self, AnalysisError> {
        self.validate()?;
        vector.validate()?;
        let mut result = Self::zero();
        for mask in 0..EXTERIOR3_COMPONENTS {
            let coefficient = self.coefficients[mask];
            for axis in 0..3 {
                let bit = 1usize << axis;
                if mask & bit == 0 {
                    continue;
                }
                let lower = (mask & (bit - 1)).count_ones();
                let sign = if lower & 1 == 0 { 1.0 } else { -1.0 };
                result.coefficients[mask ^ bit] += sign * vector.data[axis] * coefficient;
            }
        }
        Ok(result)
    }

    /// Euclidean, positively oriented Hodge star.
    pub fn hodge_star(self) -> Result<Self, AnalysisError> {
        self.validate()?;
        let mut result = Self::zero();
        for mask in 0..EXTERIOR3_COMPONENTS {
            let complement = 0b111 ^ mask;
            let (_, sign) = wedge_basis(mask, complement);
            result.coefficients[complement] += sign * self.coefficients[mask];
        }
        Ok(result)
    }
}

fn wedge_basis(left: usize, right: usize) -> (usize, f64) {
    let mut inversions = 0_u32;
    for left_axis in 0..3 {
        if left & (1 << left_axis) == 0 {
            continue;
        }
        inversions += (right & ((1 << left_axis) - 1)).count_ones();
    }
    (left | right, if inversions & 1 == 0 { 1.0 } else { -1.0 })
}

pub fn permutation_parity(permutation: &[usize]) -> Result<i8, AnalysisError> {
    for (index, value) in permutation.iter().enumerate() {
        if *value >= permutation.len()
            || permutation[index + 1..].iter().any(|other| other == value)
        {
            return Err(AnalysisError::InvalidDomain);
        }
    }
    let mut inversions = 0usize;
    for left in 0..permutation.len() {
        for right in left + 1..permutation.len() {
            inversions += usize::from(permutation[left] > permutation[right]);
        }
    }
    Ok(if inversions & 1 == 0 { 1 } else { -1 })
}

pub fn determinant_from_wedge(columns: [Vector<3>; 3]) -> Result<f64, AnalysisError> {
    let mut forms = [Exterior3::zero(); 3];
    for column in 0..3 {
        for row in 0..3 {
            forms[column].coefficients[1 << row] = columns[column].data[row];
        }
    }
    Ok(forms[0].wedge(forms[1])?.wedge(forms[2])?.coefficients[0b111])
}

pub fn pullback_one_form(
    derivative: &LinearMap<3, 3>,
    one_form: Vector<3>,
) -> Result<Vector<3>, AnalysisError> {
    derivative.transpose().apply(one_form)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wedge_is_graded_commutative_and_associative() {
        let e1 = Exterior3::basis(0b001, 1.0);
        let e2 = Exterior3::basis(0b010, 1.0);
        let e3 = Exterior3::basis(0b100, 1.0);
        assert_eq!(e1.wedge(e1).unwrap(), Exterior3::zero());
        assert_eq!(
            e1.wedge(e2).unwrap(),
            e2.wedge(e1).unwrap().scale(-1.0).unwrap()
        );
        assert_eq!(
            e1.wedge(e2).unwrap().wedge(e3).unwrap(),
            e1.wedge(e2.wedge(e3).unwrap()).unwrap()
        );
    }

    #[test]
    fn determinant_and_hodge_identities_hold() {
        let columns = [
            Vector::new([2.0, 0.0, 0.0]),
            Vector::new([1.0, 3.0, 0.0]),
            Vector::new([0.0, 2.0, 4.0]),
        ];
        assert_eq!(determinant_from_wedge(columns).unwrap(), 24.0);

        for mask in 0..8 {
            let blade = Exterior3::basis(mask, 1.0);
            let twice = blade.hodge_star().unwrap().hodge_star().unwrap();
            let grade = mask.count_ones();
            let sign = if (grade * (3 - grade)) & 1 == 0 {
                1.0
            } else {
                -1.0
            };
            assert_eq!(twice, blade.scale(sign).unwrap());
        }
    }

    #[test]
    fn permutation_and_pullback_are_correct() {
        assert_eq!(permutation_parity(&[2, 0, 1]), Ok(1));
        assert_eq!(permutation_parity(&[1, 0, 2]), Ok(-1));
        assert_eq!(
            permutation_parity(&[0, 0, 1]),
            Err(AnalysisError::InvalidDomain)
        );

        let derivative = LinearMap::new([[2.0, 0.0, 0.0], [1.0, 3.0, 0.0], [0.0, 0.0, 4.0]]);
        let pulled = pullback_one_form(&derivative, Vector::new([1.0, 2.0, 3.0])).unwrap();
        assert_eq!(pulled, Vector::new([4.0, 6.0, 12.0]));
    }
}
