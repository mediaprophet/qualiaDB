//! PGA Motor encoder for GPU transfer
//!
//! Encodes PGA Motor structures into GPU-compatible byte arrays with proper alignment.

use bytemuck::{Pod, Zeroable};

/// GPU-compatible Motor representation (64 bytes with explicit padding)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Motor {
    // Rotor components (scalar + bivectors e12, e31, e23)
    pub rotor_scalar: f32,
    pub rotor_e12: f32,
    pub rotor_e31: f32,
    pub rotor_e23: f32,
    // Translator components (scalar + vector e1, e2, e3)
    pub trans_scalar: f32,
    pub trans_e1: f32,
    pub trans_e2: f32,
    pub trans_e3: f32,
    // Padding to make struct 64 bytes for GPU alignment
    pub _pad: [f32; 8],
}

// Compile-time assertion to ensure Motor is 64 bytes
const _: [(); 64] = [(); std::mem::size_of::<Motor>()];

impl Motor {
    /// Create a new identity motor
    pub fn identity() -> Self {
        Self {
            rotor_scalar: 1.0,
            rotor_e12: 0.0,
            rotor_e31: 0.0,
            rotor_e23: 0.0,
            trans_scalar: 1.0,
            trans_e1: 0.0,
            trans_e2: 0.0,
            trans_e3: 0.0,
            _pad: [0.0; 8],
        }
    }
}

/// Motor encoder for batch GPU transfer
pub struct MotorEncoder {
    motors: Vec<Motor>,
}

impl MotorEncoder {
    /// Create a new motor encoder
    pub fn new() -> Self {
        Self { motors: Vec::new() }
    }

    /// Add a motor to the encoder
    pub fn add_motor(&mut self, motor: Motor) {
        self.motors.push(motor);
    }

    /// Encode motors into a byte array for GPU transfer
    pub fn encode(&self) -> Vec<u8> {
        bytemuck::cast_slice(&self.motors).to_vec()
    }

    /// Get the number of motors
    pub fn len(&self) -> usize {
        self.motors.len()
    }

    /// Check if the encoder is empty
    pub fn is_empty(&self) -> bool {
        self.motors.is_empty()
    }

    /// Clear all motors
    pub fn clear(&mut self) {
        self.motors.clear();
    }
}

impl Default for MotorEncoder {
    fn default() -> Self {
        Self::new()
    }
}

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
