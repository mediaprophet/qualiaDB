//! Math module for PGA Motor encoding and buffer alignment

pub mod buffer_alignment;
pub mod motor_encoder;

pub use buffer_alignment::{AlignedBufferF32, RenderQuin};
pub use motor_encoder::{Motor, MotorEncoder};
