//! 3D Geometric Algebra (Cl_3,0) SIMD Kernel
//! Enforces zero-allocation, hardware-accelerated multivector math.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::*;
use std::sync::OnceLock;
use std::ops::{Add, Sub, Neg};

/// Basis mapping for Cl_3,0 (8 dimensions):
/// 0: 1 (Scalar)
/// 1: e1 (Vector)
/// 2: e2 (Vector)
/// 3: e3 (Vector)
/// 4: e12 (Bivector)
/// 5: e13 (Bivector)
/// 6: e23 (Bivector)
/// 7: e123 (Pseudoscalar)

pub static GA_SIMD_KERNEL: OnceLock<GaKernel> = OnceLock::new();

pub struct GaKernel {
    pub has_avx2: bool,
}

impl GaKernel {
    pub fn init() -> Self {
        Self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            has_avx2: is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma"),
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            has_avx2: false,
        }
    }
}

const GEOMETRIC_PRODUCT_TABLE: [[(usize, f32); 8]; 8] = [
    // e_j = 1
    [(0, 1.), (1, 1.), (2, 1.), (3, 1.), (4, 1.), (5, 1.), (6, 1.), (7, 1.)],
    // e_j = e1
    [(1, 1.), (0, 1.), (4, -1.), (5, -1.), (2, -1.), (3, -1.), (7, 1.), (6, 1.)],
    // e_j = e2
    [(2, 1.), (4, 1.), (0, 1.), (6, -1.), (1, 1.), (7, -1.), (3, -1.), (5, -1.)],
    // e_j = e3
    [(3, 1.), (5, 1.), (6, 1.), (0, 1.), (7, 1.), (1, 1.), (2, 1.), (4, 1.)],
    // e_j = e12
    [(4, 1.), (2, 1.), (1, -1.), (7, 1.), (0, -1.), (6, 1.), (5, -1.), (3, -1.)],
    // e_j = e13
    [(5, 1.), (3, 1.), (7, -1.), (1, -1.), (6, -1.), (0, -1.), (4, 1.), (2, 1.)],
    // e_j = e23
    [(6, 1.), (7, 1.), (3, 1.), (2, -1.), (5, 1.), (4, -1.), (0, -1.), (1, -1.)],
    // e_j = e123
    [(7, 1.), (6, 1.), (5, -1.), (4, 1.), (3, -1.), (2, 1.), (1, -1.), (0, -1.)],
];

const OUTER_PRODUCT_TABLE: [[(usize, f32); 8]; 8] = [
    // 1 wedge e_j = e_j
    [(0, 1.), (1, 1.), (2, 1.), (3, 1.), (4, 1.), (5, 1.), (6, 1.), (7, 1.)],
    // e1 wedge e_j
    [(1, 1.), (0, 0.), (4, -1.), (5, -1.), (0, 0.), (0, 0.), (7, 1.), (0, 0.)],
    // e2 wedge e_j
    [(2, 1.), (4, 1.), (0, 0.), (6, -1.), (0, 0.), (7, -1.), (0, 0.), (0, 0.)],
    // e3 wedge e_j
    [(3, 1.), (5, 1.), (6, 1.), (0, 0.), (7, 1.), (0, 0.), (0, 0.), (0, 0.)],
    // e12 wedge e_j
    [(4, 1.), (0, 0.), (0, 0.), (7, 1.), (0, 0.), (0, 0.), (0, 0.), (0, 0.)],
    // e13 wedge e_j
    [(5, 1.), (0, 0.), (7, -1.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.)],
    // e23 wedge e_j
    [(6, 1.), (7, 1.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.)],
    // e123 wedge e_j
    [(7, 1.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.), (0, 0.)],
];

/// # Safety
/// This function requires the CPU to support the `AVX2` and `FMA` instruction sets.
/// Calling this on hardware without these features will result in undefined behavior (SIGILL).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
pub unsafe fn geometric_product_avx2(a: __m256, b: __m256) -> __m256 {
    let mut result = _mm256_setzero_ps();
    // j = 0
    let b0 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(0));
    let perm0 = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);
    let a_shuf0 = _mm256_permutevar8x32_ps(a, perm0);
    result = _mm256_fmadd_ps(a_shuf0, b0, result);

    // j = 1
    let b1 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(1));
    let perm1 = _mm256_set_epi32(6, 7, 3, 2, 5, 4, 0, 1);
    let a_shuf1 = _mm256_permutevar8x32_ps(a, perm1);
    let sign1 = _mm256_castsi256_ps(_mm256_set_epi32(0, 0, 0x80000000_u32 as i32, 0x80000000_u32 as i32, 0x80000000_u32 as i32, 0x80000000_u32 as i32, 0, 0));
    let a_signed1 = _mm256_xor_ps(a_shuf1, sign1);
    result = _mm256_fmadd_ps(a_signed1, b1, result);

    // j = 2
    let b2 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(2));
    let perm2 = _mm256_set_epi32(5, 3, 7, 1, 6, 0, 4, 2);
    let a_shuf2 = _mm256_permutevar8x32_ps(a, perm2);
    let sign2 = _mm256_castsi256_ps(_mm256_set_epi32(0x80000000_u32 as i32, 0x80000000_u32 as i32, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0, 0, 0));
    let a_signed2 = _mm256_xor_ps(a_shuf2, sign2);
    result = _mm256_fmadd_ps(a_signed2, b2, result);

    // j = 3
    let b3 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(3));
    let perm3 = _mm256_set_epi32(4, 2, 1, 7, 0, 6, 5, 3);
    let a_shuf3 = _mm256_permutevar8x32_ps(a, perm3);
    result = _mm256_fmadd_ps(a_shuf3, b3, result);

    // j = 4
    let b4 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(4));
    let perm4 = _mm256_set_epi32(3, 5, 6, 0, 7, 1, 2, 4);
    let a_shuf4 = _mm256_permutevar8x32_ps(a, perm4);
    let sign4 = _mm256_castsi256_ps(_mm256_set_epi32(0, 0, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0x80000000_u32 as i32));
    let a_signed4 = _mm256_xor_ps(a_shuf4, sign4);
    result = _mm256_fmadd_ps(a_signed4, b4, result);

    // j = 5
    let b5 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(5));
    let perm5 = _mm256_set_epi32(2, 4, 0, 6, 1, 7, 3, 5);
    let a_shuf5 = _mm256_permutevar8x32_ps(a, perm5);
    let sign5 = _mm256_castsi256_ps(_mm256_set_epi32(0x80000000_u32 as i32, 0x80000000_u32 as i32, 0, 0, 0, 0, 0x80000000_u32 as i32, 0x80000000_u32 as i32));
    let a_signed5 = _mm256_xor_ps(a_shuf5, sign5);
    result = _mm256_fmadd_ps(a_signed5, b5, result);

    // j = 6
    let b6 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(6));
    let perm6 = _mm256_set_epi32(1, 0, 4, 5, 2, 3, 7, 6);
    let a_shuf6 = _mm256_permutevar8x32_ps(a, perm6);
    let sign6 = _mm256_castsi256_ps(_mm256_set_epi32(0, 0, 0, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0x80000000_u32 as i32, 0x80000000_u32 as i32));
    let a_signed6 = _mm256_xor_ps(a_shuf6, sign6);
    result = _mm256_fmadd_ps(a_signed6, b6, result);

    // j = 7
    let b7 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(7));
    let perm7 = _mm256_set_epi32(0, 1, 2, 3, 4, 5, 6, 7);
    let a_shuf7 = _mm256_permutevar8x32_ps(a, perm7);
    let sign7 = _mm256_castsi256_ps(_mm256_set_epi32(0, 0, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0, 0x80000000_u32 as i32, 0x80000000_u32 as i32));
    let a_signed7 = _mm256_xor_ps(a_shuf7, sign7);
    result = _mm256_fmadd_ps(a_signed7, b7, result);

    result
}

// -----------------------------------------------------------------------------
// Fallback / Dispatcher
// -----------------------------------------------------------------------------

pub fn multivector_geometric_product(a: &[f32; 8], b: &[f32; 8]) -> [f32; 8] {
    let kernel = GA_SIMD_KERNEL.get_or_init(GaKernel::init);

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if kernel.has_avx2 {
        unsafe {
            let a_simd = _mm256_loadu_ps(a.as_ptr());
            let b_simd = _mm256_loadu_ps(b.as_ptr());
            let res_simd = geometric_product_avx2(a_simd, b_simd);
            let mut out = [0.0; 8];
            _mm256_storeu_ps(out.as_mut_ptr(), res_simd);
            return out;
        }
    }

    let mut out = [0.0; 8];
    for j in 0..8 {
        let b_j = b[j];
        if b_j == 0.0 { continue; }
        for i in 0..8 {
            let a_i = a[i];
            if a_i == 0.0 { continue; }
            let (res_idx, sign) = GEOMETRIC_PRODUCT_TABLE[j][i];
            out[res_idx] += a_i * b_j * sign;
        }
    }
    out
}

pub fn multivector_outer_product(a: &[f32; 8], b: &[f32; 8]) -> [f32; 8] {
    // For now, outer product is only scalar.
    let mut out = [0.0; 8];
    for j in 0..8 {
        let b_j = b[j];
        if b_j == 0.0 { continue; }
        for i in 0..8 {
            let a_i = a[i];
            if a_i == 0.0 { continue; }
            let (res_idx, sign) = OUTER_PRODUCT_TABLE[j][i];
            if sign != 0.0 {
                out[res_idx] += a_i * b_j * sign;
            }
        }
    }
    out
}

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
        let mag = (self.components[0] * self.components[0] +
                   self.components[1] * self.components[1] +
                   self.components[2] * self.components[2] +
                   self.components[3] * self.components[3]).sqrt();
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

pub fn geometric_product(a: &Multivector, b: &Multivector) -> Multivector {
    let coeffs = multivector_geometric_product(&a.coeffs, &b.coeffs);
    Multivector { coeffs, grade_mask: a.grade_mask | b.grade_mask }
}

pub fn outer_product(a: &Multivector, b: &Multivector) -> Multivector {
    let coeffs = multivector_outer_product(&a.coeffs, &b.coeffs);
    Multivector { coeffs, grade_mask: a.grade_mask ^ b.grade_mask }
}

pub fn rotor_from_angle_axis(angle: f32, axis: [f32; 3]) -> Rotor {
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

pub fn apply_rotor(rotor: &Rotor, vector: &[f32; 3]) -> [f32; 3] {
    let rotor_mv = Multivector::from_rotor(rotor);
    let vector_mv = Multivector::from_vector(vector);
    let rotor_reverse = rotor_mv.reverse();
    let result = geometric_product(&rotor_mv, &vector_mv);
    let final_result = geometric_product(&result, &rotor_reverse);
    final_result.to_vector()
}

pub fn translator_from_displacement(displacement: [f32; 3]) -> Translator {
    Translator {
        components: [
            1.0,
            displacement[0] * 0.5,
            displacement[1] * 0.5,
            displacement[2] * 0.5,
        ],
    }
}

pub fn apply_translator(translator: &Translator, vector: &[f32; 3]) -> [f32; 3] {
    let trans_mv = Multivector::from_translator(translator);
    let vector_mv = Multivector::from_vector(vector);
    let trans_reverse = trans_mv.reverse();
    let result = geometric_product(&trans_mv, &vector_mv);
    let final_result = geometric_product(&result, &trans_reverse);
    final_result.to_vector()
}

pub fn is_simd_available() -> bool {
    GA_SIMD_KERNEL.get_or_init(GaKernel::init).has_avx2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: f32, y: f32, z: f32) -> [f32; 8] {
        [0.0, x, y, z, 0.0, 0.0, 0.0, 0.0]
    }

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
    fn test_geometric_product_vectors() {
        let a = vec3(1.0, 2.0, 3.0);
        let b = vec3(4.0, 5.0, 6.0);
        let c = multivector_geometric_product(&a, &b);
        assert_eq!(c[0], 32.0, "Scalar part should equal the dot product");
        assert_eq!(c[4], -3.0);
        assert_eq!(c[5], -6.0);
        assert_eq!(c[6], -3.0);
        assert_eq!(c[1], 0.0);
        assert_eq!(c[2], 0.0);
        assert_eq!(c[3], 0.0);
        assert_eq!(c[7], 0.0);
    }

    #[test]
    fn test_outer_product_self_is_zero() {
        let a = vec3(2.0, -1.0, 4.0);
        let a_squared = multivector_geometric_product(&a, &a);
        assert_eq!(a_squared[0], 4.0 + 1.0 + 16.0);
        for i in 1..8 {
            assert_eq!(a_squared[i], 0.0, "Component {} must be zero", i);
        }
    }

    #[test]
    fn test_outer_product() {
        let a = Multivector::vector(1.0, 0.0, 0.0);
        let b = Multivector::vector(0.0, 1.0, 0.0);
        let result = outer_product(&a, &b);
        assert!(result.has_grade(Grade::Bivector));
        assert_eq!(result.coeffs[4], 1.0);
    }

    #[test]
    fn test_rotor_creation() {
        let rotor = rotor_from_angle_axis(std::f32::consts::PI / 2.0, [0.0, 0.0, 1.0]);
        let vector = [1.0, 0.0, 0.0];
        let rotated = apply_rotor(&rotor, &vector);
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
        assert_eq!(reversed.coeffs[4], -1.0);
        assert_eq!(reversed.coeffs[5], -2.0);
        assert_eq!(reversed.coeffs[6], -3.0);
    }

    #[test]
    fn test_simd_availability() {
        let available = is_simd_available();
        println!("SIMD available: {}", available);
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
