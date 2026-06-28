use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct HardwareCapabilityMatrix {
    pub supports_f64: bool,
    pub subgroup_size: Option<u32>,
    pub supports_coopmat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoweringPolicy64Bit {
    Native,
    PairedU32Emulation,
}

use crate::wgsl_forge::Schedule;

#[derive(Debug, Clone)]
pub struct LoweringContext {
    pub capabilities: HardwareCapabilityMatrix,
    pub schedule: Schedule,
}

impl LoweringContext {
    pub fn new(capabilities: HardwareCapabilityMatrix, schedule: Schedule) -> Self {
        Self { capabilities, schedule }
    }

    pub fn policy_64bit(&self) -> LoweringPolicy64Bit {
        if self.capabilities.supports_f64 {
            LoweringPolicy64Bit::Native
        } else {
            LoweringPolicy64Bit::PairedU32Emulation
        }
    }
}
