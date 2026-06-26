//! Geometric Algebra Module
//! 
//! High-performance geometric algebra operations for QualiaDB
//! Consolidates P2/P3 extensions into unified framework

use crate::q_hash;

pub mod simd_kernel;

// Re-export main types for convenience
pub use simd_kernel::{
    Multivector, Rotor, Translator, Grade,
    geometric_product, outer_product, rotor_from_angle_axis, apply_rotor,
    translator_from_displacement, apply_translator, is_simd_available,
};

/// Geometric Algebra constants and utilities
pub mod constants {
    /// Epsilon for floating point comparisons
    pub const GA_EPSILON: f32 = 1e-6;
    
    /// Maximum dimension supported
    pub const MAX_DIMENSION: usize = 4;
    
    /// Number of components for 3D GA
    pub const GA3D_COMPONENTS: usize = 8;
    
    /// Number of components for 4D GA
    pub const GA4D_COMPONENTS: usize = 16;
}

/// Geometric Algebra utilities
pub mod utils {
    use super::*;
    
    /// Convert degrees to radians
    pub fn deg_to_rad(degrees: f32) -> f32 {
        degrees * std::f32::consts::PI / 180.0
    }
    
    /// Convert radians to degrees
    pub fn rad_to_deg(radians: f32) -> f32 {
        radians * 180.0 / std::f32::consts::PI
    }
    
    /// Normalize vector
    pub fn normalize_vector(v: &[f32; 3]) -> [f32; 3] {
        let mag = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if mag > constants::GA_EPSILON {
            [v[0] / mag, v[1] / mag, v[2] / mag]
        } else {
            *v
        }
    }
    
    /// Cross product of two vectors
    pub fn cross_product(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    
    /// Dot product of two vectors
    pub fn dot_product(a: &[f32; 3], b: &[f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
    
    /// Angle between two vectors
    pub fn angle_between_vectors(a: &[f32; 3], b: &[f32; 3]) -> f32 {
        let mag_a = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        let mag_b = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
        
        if mag_a > constants::GA_EPSILON && mag_b > constants::GA_EPSILON {
            let cos_angle = dot_product(a, b) / (mag_a * mag_b);
            cos_angle.clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }
}

/// Geometric Algebra operations for QualiaDB integration
pub mod qualia_integration {
    use super::*;
    use crate::NQuin;
    
    /// Convert multivector to NQuin representation
    pub fn multivector_to_quin(mv: &Multivector, context: u64) -> NQuin {
        // Pack multivector coefficients into NQuin
        // This is a simplified packing - full implementation would handle all grades
        
        let scalar_bytes = mv.get_scalar().to_le_bytes();
        let scalar_hex = format!("{:02x}{:02x}{:02x}{:02x}", scalar_bytes[0], scalar_bytes[1], scalar_bytes[2], scalar_bytes[3]);
        let scalar_hash = q_hash(&scalar_hex);
        let vector_bytes: Vec<u8> = mv.to_vector().iter().flat_map(|f| f.to_le_bytes().to_vec()).collect();
        let vector_hex: String = vector_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let vector_hash = q_hash(&vector_hex);
        
        // Combine hashes for multivector representation
        let combined_hash = scalar_hash.wrapping_mul(31).wrapping_add(vector_hash);
        
        NQuin {
            subject: combined_hash,
            predicate: q_hash("q42:multivector"),
            object: q_hash("geometric_algebra"),
            context,
            metadata: mv.grade_mask() as u64,
            parity: 0,
        }
    }
    
    /// Extract multivector from NQuin
    pub fn quin_to_multivector(quin: &NQuin) -> Option<Multivector> {
        // This is a simplified extraction - full implementation would
        // reconstruct the multivector from the stored hash
        if quin.predicate == q_hash("q42:multivector") {
            Some(Multivector::scalar(0.0)) // Placeholder
        } else {
            None
        }
    }
    
    /// Create geometric operation Quin
    pub fn geometric_operation_quin(
        operation: &str,
        operand_a: u64,
        operand_b: u64,
        context: u64,
    ) -> NQuin {
        NQuin {
            subject: operand_a,
            predicate: q_hash(&format!("q42:ga:{}", operation)),
            object: operand_b,
            context,
            metadata: q_hash("geometric_operation"),
            parity: 0,
        }
    }
}

/// Performance benchmarks for geometric algebra operations
pub mod benchmarks {
    use super::*;
    use std::time::Instant;
    
    /// Benchmark geometric product performance
    pub fn benchmark_geometric_product(iterations: usize) -> (f64, bool) {
        let a = Multivector::vector(1.0, 2.0, 3.0);
        let b = Multivector::vector(4.0, 5.0, 6.0);
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = geometric_product(&a, &b);
        }
        
        let duration = start.elapsed();
        let ops_per_second = iterations as f64 / duration.as_secs_f64();
        
        (ops_per_second, is_simd_available())
    }
    
    /// Benchmark rotor application performance
    pub fn benchmark_rotor_application(iterations: usize) -> (f64, bool) {
        let rotor = rotor_from_angle_axis(std::f32::consts::PI / 4.0, [0.0, 0.0, 1.0]);
        let vector = [1.0, 0.0, 0.0];
        
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _result = apply_rotor(&rotor, &vector);
        }
        
        let duration = start.elapsed();
        let ops_per_second = iterations as f64 / duration.as_secs_f64();
        
        (ops_per_second, is_simd_available())
    }
    
    /// Run comprehensive benchmarks
    pub fn run_benchmarks() {
        const ITERATIONS: usize = 1_000_000;
        
        println!("Geometric Algebra Performance Benchmarks");
        println!("==========================================");
        
        let (gp_ops, simd_enabled) = benchmark_geometric_product(ITERATIONS);
        println!("Geometric Product: {:.2} ops/sec (SIMD: {})", gp_ops, simd_enabled);
        
        let (rotor_ops, _) = benchmark_rotor_application(ITERATIONS);
        println!("Rotor Application: {:.2} ops/sec", rotor_ops);
        
        println!("SIMD Available: {}", is_simd_available());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use super::utils::*;
    
    #[test]
    fn test_utility_functions() {
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];
        
        let cross = cross_product(&v1, &v2);
        assert_eq!(cross, [0.0, 0.0, 1.0]);
        
        let dot = dot_product(&v1, &v2);
        assert_eq!(dot, 0.0);
        
        let angle = angle_between_vectors(&v1, &v2);
        assert!((angle - std::f32::consts::PI / 2.0).abs() < constants::GA_EPSILON);
    }
    
    #[test]
    fn test_degree_radian_conversion() {
        let deg = 180.0;
        let rad = deg_to_rad(deg);
        assert!((rad - std::f32::consts::PI).abs() < constants::GA_EPSILON);
        
        let back_to_deg = rad_to_deg(rad);
        assert!((back_to_deg - deg).abs() < constants::GA_EPSILON);
    }
    
    #[test]
    fn test_vector_normalization() {
        let v = [3.0, 4.0, 0.0];
        let normalized = normalize_vector(&v);
        let mag = (normalized[0].powi(2) + normalized[1].powi(2) + normalized[2].powi(2)).sqrt();
        assert!((mag - 1.0).abs() < constants::GA_EPSILON);
    }
    
    #[test]
    fn test_qualia_integration() {
        let mv = Multivector::vector(1.0, 2.0, 3.0);
        let context = q_hash("test_context");
        
        let quin = qualia_integration::multivector_to_quin(&mv, context);
        assert_eq!(quin.context, context);
        assert_eq!(quin.predicate, q_hash("q42:multivector"));
    }
    
    #[test]
    fn test_comprehensive_operations() {
        // let kernel = get_simd_kernel();
        
        // Test rotor composition
        let rotor1 = rotor_from_angle_axis(deg_to_rad(45.0), [0.0, 0.0, 1.0]);
        let rotor2 = rotor_from_angle_axis(deg_to_rad(30.0), [0.0, 1.0, 0.0]);
        
        let vector = [1.0, 0.0, 0.0];
        let rotated1 = apply_rotor(&rotor1, &vector);
        let rotated2 = apply_rotor(&rotor2, &rotated1);
        
        // Verify the result is a unit vector
        let mag = (rotated2[0].powi(2) + rotated2[1].powi(2) + rotated2[2].powi(2)).sqrt();
        assert!((mag - 1.0).abs() < constants::GA_EPSILON);
        
        // Test translator composition
        let trans1 = translator_from_displacement([1.0, 2.0, 3.0]);
        let trans2 = translator_from_displacement([4.0, 5.0, 6.0]);
        
        let origin = [0.0, 0.0, 0.0];
        let translated1 = apply_translator(&trans1, &origin);
        let translated2 = apply_translator(&trans2, &translated1);
        
        assert_eq!(translated2, [5.0, 7.0, 9.0]);
    }
}
