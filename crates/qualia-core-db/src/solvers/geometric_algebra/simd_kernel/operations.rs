//! High-level geometric-algebra operations over the value types: products that
//! track grades, and rotor/translator construction and application.

use super::simd_backend::{
    multivector_geometric_product, multivector_outer_product, GaKernel, GA_SIMD_KERNEL,
};
use super::types::{Grade, Multivector, Rotor, Translator};

pub fn geometric_product(a: &Multivector, b: &Multivector) -> Multivector {
    let coeffs = multivector_geometric_product(&a.coeffs, &b.coeffs);
    Multivector {
        coeffs,
        grade_mask: a.grade_mask | b.grade_mask,
    }
}

pub fn outer_product(a: &Multivector, b: &Multivector) -> Multivector {
    let coeffs = multivector_outer_product(&a.coeffs, &b.coeffs);
    // Compute grade_mask from the actual non-zero output coefficients.
    // XOR of input grade masks is wrong: e.g. vector∧vector = bivector, but
    // 0b0010 ^ 0b0010 = 0b0000 (no grade bits set), whereas the correct result
    // is a bivector so bit 0b0100 must be set.
    let mut grade_mask: u8 = 0;
    if coeffs[0].abs() > f32::EPSILON {
        grade_mask |= Grade::Scalar as u8;
    }
    if coeffs[1].abs() > f32::EPSILON
        || coeffs[2].abs() > f32::EPSILON
        || coeffs[3].abs() > f32::EPSILON
    {
        grade_mask |= Grade::Vector as u8;
    }
    if coeffs[4].abs() > f32::EPSILON
        || coeffs[5].abs() > f32::EPSILON
        || coeffs[6].abs() > f32::EPSILON
    {
        grade_mask |= Grade::Bivector as u8;
    }
    if coeffs[7].abs() > f32::EPSILON {
        grade_mask |= Grade::Trivector as u8;
    }
    Multivector { coeffs, grade_mask }
}

pub fn rotor_from_angle_axis(angle: f32, axis: [f32; 3]) -> Rotor {
    let half_angle = angle * 0.5;
    let sin_half = half_angle.sin();
    let cos_half = half_angle.cos();
    // Rotor R = exp(-θ/2 · B) = cos(θ/2) - sin(θ/2)·B, where B is the unit bivector
    // dual to the rotation axis in Cl_3,0:
    //   *(nx·e1) = nx·(e2∧e3) = nx·e23
    //   *(ny·e2) = ny·(e3∧e1) = ny·e31 = -ny·e13
    //   *(nz·e3) = nz·(e1∧e2) = nz·e12
    // So B = nz·e12 - ny·e13 + nx·e23, and R has bivector coefficients:
    //   e12 (components[1] → coeffs[4]): -sin(θ/2) · nz
    //   e13 (components[2] → coeffs[5]): +sin(θ/2) · ny
    //   e23 (components[3] → coeffs[6]): -sin(θ/2) · nx
    Rotor {
        components: [
            cos_half,
            -axis[2] * sin_half, // e12 coeff: -sin · nz
            axis[1] * sin_half,  // e13 coeff: +sin · ny
            -axis[0] * sin_half, // e23 coeff: -sin · nx
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
    // In Cl_3,0 (non-degenerate 3D GA), true translation versors do not exist.
    // The Translator struct encodes a half-displacement: components[1..=3] = d/2.
    // `displacement()` recovers the full displacement as 2 * components[1..=3].
    // Translation is applied by adding the displacement directly.
    // NOTE: The sandwich-product approach (T·v·T̃) used previously is incorrect in
    // Cl_3,0 because `reverse()` does not negate the vector (grade-1) part, so the
    // sandwich returns zero for a zero input vector and fails in general.
    let d = translator.displacement();
    [vector[0] + d[0], vector[1] + d[1], vector[2] + d[2]]
}

pub fn is_simd_available() -> bool {
    GA_SIMD_KERNEL.get_or_init(GaKernel::init).has_avx2
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_simd_availability() {
        let available = is_simd_available();
        println!("SIMD available: {}", available);
    }
}
