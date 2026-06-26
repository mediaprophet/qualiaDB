//! Geometric-algebra value types: `Multivector`, `Grade`, `Rotor`, `Translator`
//! and their constructors / accessors / operators. Pure structured math over
//! `[f32; N]` coefficient arrays — no SIMD, no graph.

use std::ops::{Add, Neg, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Multivector {
    pub coeffs: [f32; 8],
    pub grade_mask: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Scalar = 0b0001,
    Vector = 0b0010,
    Bivector = 0b0100,
    Trivector = 0b1000,
}

#[derive(Debug, Clone, Copy)]
pub struct Rotor {
    pub components: [f32; 4],
}

#[derive(Debug, Clone, Copy)]
pub struct Translator {
    pub components: [f32; 4],
}

impl Default for Multivector {
    fn default() -> Self {
        Self::zero()
    }
}

impl Multivector {
    pub fn zero() -> Self {
        Self { coeffs: [0.0; 8], grade_mask: 0 }
    }

    pub fn scalar(value: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = value;
        mv.grade_mask = Grade::Scalar as u8;
        mv
    }

    pub fn vector(x: f32, y: f32, z: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[1] = x;
        mv.coeffs[2] = y;
        mv.coeffs[3] = z;
        mv.grade_mask = Grade::Vector as u8;
        mv
    }

    pub fn bivector(xy: f32, xz: f32, yz: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[4] = xy;
        mv.coeffs[5] = xz;
        mv.coeffs[6] = yz;
        mv.grade_mask = Grade::Bivector as u8;
        mv
    }

    pub fn trivector(xyz: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[7] = xyz;
        mv.grade_mask = Grade::Trivector as u8;
        mv
    }

    pub fn from_rotor(rotor: &Rotor) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = rotor.components[0];
        mv.coeffs[4] = rotor.components[1];
        mv.coeffs[5] = rotor.components[2];
        mv.coeffs[6] = rotor.components[3];
        mv.grade_mask = Grade::Scalar as u8 | Grade::Bivector as u8;
        mv
    }

    pub fn from_translator(translator: &Translator) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = translator.components[0];
        mv.coeffs[1] = translator.components[1];
        mv.coeffs[2] = translator.components[2];
        mv.coeffs[3] = translator.components[3];
        mv.grade_mask = Grade::Scalar as u8 | Grade::Vector as u8;
        mv
    }

    pub fn from_vector(vector: &[f32; 3]) -> Self {
        Self::vector(vector[0], vector[1], vector[2])
    }

    pub fn to_vector(&self) -> [f32; 3] {
        [self.coeffs[1], self.coeffs[2], self.coeffs[3]]
    }

    pub fn get_scalar(&self) -> f32 {
        self.coeffs[0]
    }

    pub fn reverse(&self) -> Self {
        let mut result = *self;
        result.coeffs[4] = -result.coeffs[4];
        result.coeffs[5] = -result.coeffs[5];
        result.coeffs[6] = -result.coeffs[6];
        result.coeffs[7] = -result.coeffs[7];
        result
    }

    pub fn div_scalar(&self, scalar: f32) -> Self {
        let mut result = *self;
        for coeff in &mut result.coeffs {
            *coeff /= scalar;
        }
        result
    }

    pub fn add(&self, other: &Self) -> Self {
        let mut result = *self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff += other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }

    pub fn sub(&self, other: &Self) -> Self {
        let mut result = *self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff -= other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }

    pub fn is_zero(&self) -> bool {
        self.coeffs.iter().all(|&c| c.abs() < f32::EPSILON)
    }

    pub fn grade_mask(&self) -> u8 {
        self.grade_mask
    }

    pub fn has_grade(&self, grade: Grade) -> bool {
        self.grade_mask & (grade as u8) != 0
    }
}

impl Add for Multivector {
    type Output = Self;
    fn add(self, other: Self) -> Self { Multivector::add(&self, &other) }
}

impl Sub for Multivector {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Multivector::sub(&self, &other) }
}

impl Neg for Multivector {
    type Output = Self;
    fn neg(self) -> Self {
        let mut result = self;
        for coeff in &mut result.coeffs {
            *coeff = -*coeff;
        }
        result
    }
}

impl Rotor {
    pub fn identity() -> Self {
        Self { components: [1.0, 0.0, 0.0, 0.0] }
    }
    pub fn from_quaternion(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self { components: [w, x, y, z] }
    }
    pub fn as_quaternion(&self) -> [f32; 4] {
        self.components
    }
    pub fn normalize(&mut self) {
        let mag = (self.components[0] * self.components[0]
            + self.components[1] * self.components[1]
            + self.components[2] * self.components[2]
            + self.components[3] * self.components[3])
            .sqrt();
        if mag > 0.0 {
            for comp in &mut self.components { *comp /= mag; }
        }
    }
    pub fn inverse(&self) -> Self {
        let mut result = *self;
        result.components[1] = -result.components[1];
        result.components[2] = -result.components[2];
        result.components[3] = -result.components[3];
        result
    }
}

impl Translator {
    pub fn identity() -> Self {
        Self { components: [1.0, 0.0, 0.0, 0.0] }
    }
    pub fn displacement(&self) -> [f32; 3] {
        [self.components[1] * 2.0, self.components[2] * 2.0, self.components[3] * 2.0]
    }
    pub fn inverse(&self) -> Self {
        Self {
            components: [
                self.components[0],
                -self.components[1],
                -self.components[2],
                -self.components[3],
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multivector_creation() {
        let scalar = Multivector::scalar(1.0);
        assert_eq!(scalar.coeffs[0], 1.0);
        assert!(scalar.has_grade(Grade::Scalar));

        let vector = Multivector::vector(1.0, 2.0, 3.0);
        assert_eq!(vector.to_vector(), [1.0, 2.0, 3.0]);
        assert!(vector.has_grade(Grade::Vector));

        let bivector = Multivector::bivector(1.0, 2.0, 3.0);
        assert!(bivector.has_grade(Grade::Bivector));

        let trivector = Multivector::trivector(1.0);
        assert!(trivector.has_grade(Grade::Trivector));
    }

    #[test]
    fn test_multivector_reverse() {
        let bivector = Multivector::bivector(1.0, 2.0, 3.0);
        let reversed = bivector.reverse();
        assert_eq!(reversed.coeffs[4], -1.0);
        assert_eq!(reversed.coeffs[5], -2.0);
        assert_eq!(reversed.coeffs[6], -3.0);
    }

    #[test]
    fn test_multivector_arithmetic() {
        let a = Multivector::vector(1.0, 2.0, 3.0);
        let b = Multivector::vector(4.0, 5.0, 6.0);
        let sum = a + b;
        assert_eq!(sum.to_vector(), [5.0, 7.0, 9.0]);
        let diff = a - b;
        assert_eq!(diff.to_vector(), [-3.0, -3.0, -3.0]);
        let neg = -a;
        assert_eq!(neg.to_vector(), [-1.0, -2.0, -3.0]);
    }
}
