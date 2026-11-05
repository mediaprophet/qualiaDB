//! PGA Motor encoder for GPU transfer.
//!
//! Re-exports the canonical PGA Motor and MotorEncoder primitives from
//! `etalon_render::webizen_core::motor_encoder`.

pub use etalon_render::webizen_core::motor_encoder::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_size() {
        assert_eq!(std::mem::size_of::<Motor>(), 64); // 64 bytes total
    }

    #[test]
    fn test_motor_identity() {
        let motor = Motor::identity();
        assert_eq!(motor.rotor_scalar, 1.0);
        assert_eq!(motor.trans_scalar, 1.0);
    }

    #[test]
    fn test_motor_encoder() {
        let mut encoder = MotorEncoder::new();
        encoder.add_motor(Motor::identity());
        encoder.add_motor(Motor::identity());

        assert_eq!(encoder.len(), 2);
        let bytes = encoder.encode();
        assert_eq!(bytes.len(), 2 * 64); // 2 motors * 64 bytes each
    }
}
