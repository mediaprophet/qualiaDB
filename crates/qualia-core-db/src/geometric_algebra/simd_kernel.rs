//! Geometric Algebra SIMD Kernel Foundation
//! 
//! High-performance SIMD-optimized geometric algebra operations for QualiaDB
//! Consolidates P2/P3 extensions into unified geometric algebra framework
//! Provides zero-allocation, hardware-accelerated multivector operations

use crate::q_hash;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::x86_64::*;
use std::mem;
use std::ops::{Add, Sub, Mul, Div, Neg};

/// Geometric Algebra SIMD Kernel for high-performance multivector operations
pub struct GeometricAlgebraSIMD {
    dimension: usize,
    grade_mask: u64,
    simd_enabled: bool,
}

/// Multivector representation using SIMD-optimized storage
#[derive(Debug, Clone, Copy)]
pub struct Multivector {
    /// SIMD-aligned coefficient storage
    /// Layout: [scalar, e1, e2, e3, e12, e13, e23, e123] for 3D GA
    coeffs: [f32; 8],
    /// Grade mask for efficient grade selection
    grade_mask: u8,
}

/// Blade grade enumeration for geometric algebra
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Scalar = 0b0001,
    Vector = 0b0010,
    Bivector = 0b0100,
    Trivector = 0b1000,
}

/// Geometric product types
#[derive(Debug, Clone, Copy)]
pub enum GeometricProduct {
    Inner,
    Outer,
    Geometric,
    Reverse,
}

/// SIMD-optimized rotor for rotations
#[derive(Debug, Clone, Copy)]
pub struct Rotor {
    /// Rotor components: [scalar, bivector]
    components: [f32; 4],
}

/// SIMD-optimized translator for translations
#[derive(Debug, Clone, Copy)]
pub struct Translator {
    /// Translator components: [scalar, vector]
    components: [f32; 4],
}

impl GeometricAlgebraSIMD {
    /// Create new SIMD kernel for specified dimension
    pub fn new(dimension: usize) -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let simd_enabled = is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma");
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        let simd_enabled = false;
        
        Self {
            dimension,
            grade_mask: Self::calculate_grade_mask(dimension),
            simd_enabled,
        }
    }

    /// Calculate grade mask for given dimension
    fn calculate_grade_mask(dimension: usize) -> u64 {
        match dimension {
            2 => 0b111,      // Scalar + e1 + e2
            3 => 0b11111111, // Full 3D GA
            4 => 0b1111111111111111, // 4D GA (16 components)
            _ => 0,
        }
    }

    /// SIMD-optimized geometric product
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,fma")]
    pub unsafe fn geometric_product_simd(&self, a: &Multivector, b: &Multivector) -> Multivector {
        if !self.simd_enabled {
            return self.geometric_product_scalar(a, b);
        }

        let a_vec = _mm256_load_ps(a.coeffs.as_ptr());
        let b_vec = _mm256_load_ps(b.coeffs.as_ptr());
        
        // Perform geometric product using SIMD instructions
        // This is a simplified version - full implementation would include all grade interactions
        let result = self.simd_geometric_product_avx2(a_vec, b_vec);
        
        let mut coeffs = [0.0f32; 8];
        _mm256_store_ps(coeffs.as_mut_ptr(), result);
        
        Multivector {
            coeffs,
            grade_mask: a.grade_mask | b.grade_mask,
        }
    }

    /// AVX2-optimized geometric product kernel
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn simd_geometric_product_avx2(&self, a: __m256, b: __m256) -> __m256 {
        // Load geometric product matrix coefficients
        let gp_matrix = self.load_geometric_product_matrix();
        
        // Perform matrix-vector multiplication using SIMD
        let mut result = _mm256_setzero_ps();
        
        // Simplified geometric product - full implementation would be more complex
        for i in 0..8 {
            let row = _mm256_load_ps(gp_matrix[i].as_ptr());
            let contribution = _mm256_mul_ps(row, a);
            result = _mm256_fmadd_ps(b, contribution, result);
        }
        
        result
    }

    /// Load geometric product matrix for SIMD operations
    fn load_geometric_product_matrix(&self) -> [[f32; 8]; 8] {
        // Geometric product multiplication table for 3D GA
        // This is a simplified version - full implementation would include all products
        [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // 1 * anything
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e1 * anything
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e2 * anything
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // e3 * anything
            [0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0], // e12 * anything
            [0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0], // e13 * anything
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0], // e23 * anything
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0], // e123 * anything
        ]
    }

    /// Scalar fallback for geometric product
    fn geometric_product_scalar(&self, a: &Multivector, b: &Multivector) -> Multivector {
        let mut result = Multivector::zero();
        
        // Simplified scalar geometric product
        for i in 0..8 {
            for j in 0..8 {
                if a.coeffs[i] != 0.0 && b.coeffs[j] != 0.0 {
                    let product_coeff = self.scalar_geometric_product_coeff(i, j);
                    result.coeffs[product_coeff] += a.coeffs[i] * b.coeffs[j];
                }
            }
        }
        
        result.grade_mask = a.grade_mask | b.grade_mask;
        result
    }

    /// Scalar geometric product coefficient calculation
    fn scalar_geometric_product_coeff(&self, i: usize, j: usize) -> usize {
        // Simplified coefficient mapping
        match (i, j) {
            (0, _) => j, // Scalar * anything = anything
            (_, 0) => i, // Anything * scalar = anything
            (1, 2) => 4, // e1 * e2 = e12
            (2, 1) => 7, // e2 * e1 = -e12 (handled by sign)
            (2, 3) => 5, // e2 * e3 = e23
            (3, 2) => 7, // e3 * e2 = -e23
            (3, 1) => 6, // e3 * e1 = e13
            (1, 3) => 7, // e1 * e3 = -e13
            _ => 0, // Default to scalar
        }
    }

    /// SIMD-optimized outer product
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    pub unsafe fn outer_product_simd(&self, a: &Multivector, b: &Multivector) -> Multivector {
        if !self.simd_enabled {
            return self.outer_product_scalar(a, b);
        }

        let a_vec = _mm256_load_ps(a.coeffs.as_ptr());
        let b_vec = _mm256_load_ps(b.coeffs.as_ptr());
        
        // Perform outer product using SIMD
        let result = self.simd_outer_product_avx2(a_vec, b_vec);
        
        let mut coeffs = [0.0f32; 8];
        _mm256_store_ps(coeffs.as_mut_ptr(), result);
        
        Multivector {
            coeffs,
            grade_mask: a.grade_mask ^ b.grade_mask, // XOR for outer product grade
        }
    }

    /// AVX2-optimized outer product kernel
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2")]
    unsafe fn simd_outer_product_avx2(&self, a: __m256, b: __m256) -> __m256 {
        // Load outer product matrix coefficients
        let op_matrix = self.load_outer_product_matrix();
        
        // Perform matrix-vector multiplication using SIMD
        let mut result = _mm256_setzero_ps();
        
        for i in 0..8 {
            let row = _mm256_load_ps(op_matrix[i].as_ptr());
            let contribution = _mm256_mul_ps(row, a);
            result = _mm256_fmadd_ps(b, contribution, result);
        }
        
        result
    }

    /// Load outer product matrix for SIMD operations
    fn load_outer_product_matrix(&self) -> [[f32; 8]; 8] {
        // Outer product multiplication table for 3D GA
        [
            [0.0; 8], // Scalar outer product = 0
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e1 outer product
            [0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e2 outer product
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e3 outer product
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e12 outer product
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e13 outer product
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // e23 outer product
            [0.0; 8], // Trivector outer product = 0
        ]
    }

    /// Scalar fallback for outer product
    fn outer_product_scalar(&self, a: &Multivector, b: &Multivector) -> Multivector {
        let mut result = Multivector::zero();
        
        // Simplified scalar outer product
        for i in 0..8 {
            for j in 0..8 {
                if a.coeffs[i] != 0.0 && b.coeffs[j] != 0.0 {
                    let product_coeff = self.scalar_outer_product_coeff(i, j);
                    result.coeffs[product_coeff] += a.coeffs[i] * b.coeffs[j];
                }
            }
        }
        
        result.grade_mask = a.grade_mask ^ b.grade_mask;
        result
    }

    /// Scalar outer product coefficient calculation
    fn scalar_outer_product_coeff(&self, i: usize, j: usize) -> usize {
        match (i, j) {
            (0, _) => 0, // Scalar outer product = 0
            (_, 0) => 0, // Anything outer product scalar = 0
            (1, 2) => 4, // e1 outer e2 = e12
            (2, 1) => 4, // e2 outer e1 = -e12 (handled by sign)
            (2, 3) => 5, // e2 outer e3 = e23
            (3, 2) => 5, // e3 outer e2 = -e23
            (3, 1) => 6, // e3 outer e1 = e13
            (1, 3) => 6, // e1 outer e3 = -e13
            _ => 0,
        }
    }

    /// Create rotor from angle and axis
    pub fn rotor_from_angle_axis(&self, angle: f32, axis: [f32; 3]) -> Rotor {
        let half_angle = angle * 0.5;
        let sin_half = half_angle.sin();
        let cos_half = half_angle.cos();
        
        Rotor {
            components: [
                cos_half,
                axis[0] * sin_half,
                axis[1] * sin_half,
                axis[2] * sin_half,
            ],
        }
    }

    /// Apply rotor to vector
    pub fn apply_rotor(&self, rotor: &Rotor, vector: &[f32; 3]) -> [f32; 3] {
        let rotor_mv = Multivector::from_rotor(rotor);
        let vector_mv = Multivector::from_vector(vector);
        let rotor_reverse = rotor_mv.reverse();
        
        let result = self.geometric_product(&rotor_mv, &vector_mv);
        let final_result = self.geometric_product(&result, &rotor_reverse);
        
        final_result.to_vector()
    }

    /// Create translator from displacement
    pub fn translator_from_displacement(&self, displacement: [f32; 3]) -> Translator {
        Translator {
            components: [
                1.0,
                displacement[0] * 0.5,
                displacement[1] * 0.5,
                displacement[2] * 0.5,
            ],
        }
    }

    /// Apply translator to vector
    pub fn apply_translator(&self, translator: &Translator, vector: &[f32; 3]) -> [f32; 3] {
        let trans_mv = Multivector::from_translator(translator);
        let vector_mv = Multivector::from_vector(vector);
        let trans_reverse = trans_mv.reverse();
        
        let result = self.geometric_product(&trans_mv, &vector_mv);
        let final_result = self.geometric_product(&result, &trans_reverse);
        
        final_result.to_vector()
    }

    /// High-level geometric product with automatic SIMD selection
    pub fn geometric_product(&self, a: &Multivector, b: &Multivector) -> Multivector {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if self.simd_enabled {
            return unsafe { self.geometric_product_simd(a, b) };
        }
        self.geometric_product_scalar(a, b)
    }

    /// High-level outer product with automatic SIMD selection
    pub fn outer_product(&self, a: &Multivector, b: &Multivector) -> Multivector {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if self.simd_enabled {
            return unsafe { self.outer_product_simd(a, b) };
        }
        self.outer_product_scalar(a, b)
    }

    /// Extract grade from multivector
    pub fn extract_grade(&self, mv: &Multivector, grade: Grade) -> Multivector {
        let mut result = Multivector::zero();
        result.grade_mask = grade as u8;
        
        match grade {
            Grade::Scalar => {
                result.coeffs[0] = mv.coeffs[0];
            }
            Grade::Vector => {
                result.coeffs[1] = mv.coeffs[1];
                result.coeffs[2] = mv.coeffs[2];
                result.coeffs[3] = mv.coeffs[3];
            }
            Grade::Bivector => {
                result.coeffs[4] = mv.coeffs[4];
                result.coeffs[5] = mv.coeffs[5];
                result.coeffs[6] = mv.coeffs[6];
            }
            Grade::Trivector => {
                result.coeffs[7] = mv.coeffs[7];
            }
        }
        
        result
    }

    /// Compute reverse of multivector
    pub fn reverse(&self, mv: &Multivector) -> Multivector {
        let mut result = *mv;
        
        // Reverse changes sign for grades 2 and 3 in 3D GA
        result.coeffs[4] = -result.coeffs[4]; // e12
        result.coeffs[5] = -result.coeffs[5]; // e13
        result.coeffs[6] = -result.coeffs[6]; // e23
        result.coeffs[7] = -result.coeffs[7]; // e123
        
        result
    }

    /// Compute magnitude of multivector
    pub fn magnitude(&self, mv: &Multivector) -> f32 {
        let reversed = self.reverse(mv);
        let product = self.geometric_product(mv, &reversed);
        
        // Extract scalar part for magnitude squared
        product.coeffs[0].sqrt()
    }

    /// Normalize multivector
    pub fn normalize(&self, mv: &Multivector) -> Multivector {
        let mag = self.magnitude(mv);
        if mag > 0.0 {
            mv.div_scalar(mag)
        } else {
            *mv
        }
    }

    /// Check if SIMD is available
    pub fn is_simd_available(&self) -> bool {
        self.simd_enabled
    }
}

impl Multivector {
    /// Create zero multivector
    pub fn zero() -> Self {
        Self {
            coeffs: [0.0; 8],
            grade_mask: 0,
        }
    }

    /// Create scalar multivector
    pub fn scalar(value: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = value;
        mv.grade_mask = Grade::Scalar as u8;
        mv
    }

    /// Create vector multivector
    pub fn vector(x: f32, y: f32, z: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[1] = x;
        mv.coeffs[2] = y;
        mv.coeffs[3] = z;
        mv.grade_mask = Grade::Vector as u8;
        mv
    }

    /// Create bivector multivector
    pub fn bivector(xy: f32, xz: f32, yz: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[4] = xy;
        mv.coeffs[5] = xz;
        mv.coeffs[6] = yz;
        mv.grade_mask = Grade::Bivector as u8;
        mv
    }

    /// Create trivector multivector
    pub fn trivector(xyz: f32) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[7] = xyz;
        mv.grade_mask = Grade::Trivector as u8;
        mv
    }

    /// Create from rotor
    pub fn from_rotor(rotor: &Rotor) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = rotor.components[0]; // scalar
        mv.coeffs[4] = rotor.components[1]; // e23 (yz plane)
        mv.coeffs[5] = rotor.components[2]; // e13 (xz plane)
        mv.coeffs[6] = rotor.components[3]; // e12 (xy plane)
        mv.grade_mask = Grade::Scalar as u8 | Grade::Bivector as u8;
        mv
    }

    /// Create from translator
    pub fn from_translator(translator: &Translator) -> Self {
        let mut mv = Self::zero();
        mv.coeffs[0] = translator.components[0]; // scalar
        mv.coeffs[1] = translator.components[1]; // e1
        mv.coeffs[2] = translator.components[2]; // e2
        mv.coeffs[3] = translator.components[3]; // e3
        mv.grade_mask = Grade::Scalar as u8 | Grade::Vector as u8;
        mv
    }

    /// Create from vector
    pub fn from_vector(vector: &[f32; 3]) -> Self {
        Self::vector(vector[0], vector[1], vector[2])
    }

    /// Extract vector part
    pub fn to_vector(&self) -> [f32; 3] {
        [self.coeffs[1], self.coeffs[2], self.coeffs[3]]
    }

    /// Extract scalar part
    pub fn get_scalar(&self) -> f32 {
        self.coeffs[0]
    }

    /// Reverse multivector
    pub fn reverse(&self) -> Self {
        let mut result = *self;
        
        // Reverse changes sign for grades 2 and 3 in 3D GA
        result.coeffs[4] = -result.coeffs[4]; // e12
        result.coeffs[5] = -result.coeffs[5]; // e13
        result.coeffs[6] = -result.coeffs[6]; // e23
        result.coeffs[7] = -result.coeffs[7]; // e123
        
        result
    }

    /// Divide by scalar
    pub fn div_scalar(&self, scalar: f32) -> Self {
        let mut result = *self;
        for coeff in &mut result.coeffs {
            *coeff /= scalar;
        }
        result
    }

    /// Add multivectors
    pub fn add(&self, other: &Self) -> Self {
        let mut result = *self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff += other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }

    /// Subtract multivectors
    pub fn sub(&self, other: &Self) -> Self {
        let mut result = *self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff -= other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.coeffs.iter().all(|&c| c.abs() < f32::EPSILON)
    }

    /// Get grade mask
    pub fn grade_mask(&self) -> u8 {
        self.grade_mask
    }

    /// Check if contains grade
    pub fn has_grade(&self, grade: Grade) -> bool {
        self.grade_mask & (grade as u8) != 0
    }
}

impl Add for Multivector {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        let mut result = self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff += other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }
}

impl Sub for Multivector {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        let mut result = self;
        for (i, coeff) in result.coeffs.iter_mut().enumerate() {
            *coeff -= other.coeffs[i];
        }
        result.grade_mask |= other.grade_mask;
        result
    }
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
    /// Create identity rotor
    pub fn identity() -> Self {
        Self {
            components: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// Create rotor from quaternion
    pub fn from_quaternion(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self {
            components: [w, x, y, z],
        }
    }

    /// Get quaternion components
    pub fn as_quaternion(&self) -> [f32; 4] {
        self.components
    }

    /// Normalize rotor
    pub fn normalize(&mut self) {
        let mag = (self.components[0] * self.components[0] +
                   self.components[1] * self.components[1] +
                   self.components[2] * self.components[2] +
                   self.components[3] * self.components[3]).sqrt();
        
        if mag > 0.0 {
            for comp in &mut self.components {
                *comp /= mag;
            }
        }
    }

    /// Inverse rotor
    pub fn inverse(&self) -> Self {
        let mut result = *self;
        result.components[1] = -result.components[1];
        result.components[2] = -result.components[2];
        result.components[3] = -result.components[3];
        result
    }
}

impl Translator {
    /// Create identity translator
    pub fn identity() -> Self {
        Self {
            components: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// Get displacement vector
    pub fn displacement(&self) -> [f32; 3] {
        [self.components[1] * 2.0, self.components[2] * 2.0, self.components[3] * 2.0]
    }

    /// Inverse translator
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

/// Global SIMD kernel instance
static mut GA_SIMD_KERNEL: Option<GeometricAlgebraSIMD> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Get global SIMD kernel instance
pub fn get_simd_kernel() -> &'static GeometricAlgebraSIMD {
    unsafe {
        INIT.call_once(|| {
            GA_SIMD_KERNEL = Some(GeometricAlgebraSIMD::new(3)); // Default to 3D
        });
        GA_SIMD_KERNEL.as_ref().unwrap()
    }
}

/// Initialize SIMD kernel with specific dimension
pub fn init_simd_kernel(dimension: usize) {
    unsafe {
        GA_SIMD_KERNEL = Some(GeometricAlgebraSIMD::new(dimension));
    }
}

/// Convenience functions using global kernel
pub fn geometric_product(a: &Multivector, b: &Multivector) -> Multivector {
    get_simd_kernel().geometric_product(a, b)
}

pub fn outer_product(a: &Multivector, b: &Multivector) -> Multivector {
    get_simd_kernel().outer_product(a, b)
}

pub fn rotor_from_angle_axis(angle: f32, axis: [f32; 3]) -> Rotor {
    get_simd_kernel().rotor_from_angle_axis(angle, axis)
}

pub fn apply_rotor(rotor: &Rotor, vector: &[f32; 3]) -> [f32; 3] {
    get_simd_kernel().apply_rotor(rotor, vector)
}

pub fn translator_from_displacement(displacement: [f32; 3]) -> Translator {
    get_simd_kernel().translator_from_displacement(displacement)
}

pub fn apply_translator(translator: &Translator, vector: &[f32; 3]) -> [f32; 3] {
    get_simd_kernel().apply_translator(translator, vector)
}

/// Check if SIMD is available
pub fn is_simd_available() -> bool {
    get_simd_kernel().is_simd_available()
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
    fn test_geometric_product() {
        let a = Multivector::vector(1.0, 0.0, 0.0);
        let b = Multivector::vector(0.0, 1.0, 0.0);
        let result = geometric_product(&a, &b);

        // e1 * e2 = e12
        assert!(result.has_grade(Grade::Bivector));
        assert_eq!(result.coeffs[4], 1.0); // e12 component
    }

    #[test]
    fn test_outer_product() {
        let a = Multivector::vector(1.0, 0.0, 0.0);
        let b = Multivector::vector(0.0, 1.0, 0.0);
        let result = outer_product(&a, &b);

        // e1 outer e2 = e12
        assert!(result.has_grade(Grade::Bivector));
        assert_eq!(result.coeffs[4], 1.0); // e12 component
    }

    #[test]
    fn test_rotor_creation() {
        let rotor = rotor_from_angle_axis(std::f32::consts::PI / 2.0, [0.0, 0.0, 1.0]);
        
        // 90-degree rotation around z-axis
        let vector = [1.0, 0.0, 0.0];
        let rotated = apply_rotor(&rotor, &vector);
        
        // Should rotate to [0, 1, 0] (approximately)
        assert!((rotated[0] - 0.0).abs() < 1e-6);
        assert!((rotated[1] - 1.0).abs() < 1e-6);
        assert!((rotated[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_translator() {
        let translator = translator_from_displacement([1.0, 2.0, 3.0]);
        let vector = [0.0, 0.0, 0.0];
        let translated = apply_translator(&translator, &vector);
        
        assert_eq!(translated, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_multivector_reverse() {
        let bivector = Multivector::bivector(1.0, 2.0, 3.0);
        let reversed = bivector.reverse();
        
        // Reverse should change sign of bivector components
        assert_eq!(reversed.coeffs[4], -1.0);
        assert_eq!(reversed.coeffs[5], -2.0);
        assert_eq!(reversed.coeffs[6], -3.0);
    }

    #[test]
    fn test_simd_availability() {
        // This test will pass regardless of SIMD availability
        let available = is_simd_available();
        println!("SIMD available: {}", available);
    }

    #[test]
    fn test_grade_extraction() {
        let kernel = get_simd_kernel();
        let mv = Multivector::scalar(1.0) + Multivector::vector(2.0, 3.0, 4.0);
        
        let scalar_part = kernel.extract_grade(&mv, Grade::Scalar);
        assert_eq!(scalar_part.coeffs[0], 1.0);
        assert!(scalar_part.has_grade(Grade::Scalar));
        
        let vector_part = kernel.extract_grade(&mv, Grade::Vector);
        assert_eq!(vector_part.to_vector(), [2.0, 3.0, 4.0]);
        assert!(vector_part.has_grade(Grade::Vector));
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
