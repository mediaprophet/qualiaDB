use serde::{Deserialize, Serialize};

use super::intrinsics::{Intrinsic, IntrinsicClass};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct HardwareCapabilityMatrix {
    /// **Reserved scaffolding (plan §1): not yet exercised by any emitter.** No IR
    /// `ScalarType` variant emits native `f64`/`u64`; 64-bit values are carried as
    /// portable paired-`u32` words (`ScalarType::U64Words`). This flag and the
    /// [`LoweringContext::policy_64bit`] policy it drives exist so the native-64-bit
    /// lowering can be wired the moment a kernel needs native 64-bit arithmetic — do
    /// not assume any generated shader consults it today.
    pub supports_f64: bool,
    pub subgroup_size: Option<u32>,
    /// Cooperative-matrix / Tensor-core matrix-multiply-accumulate support.
    pub supports_coopmat: bool,
    /// Ray-query / RT-core support (hardware ray-triangle intersection).
    pub supports_rt_cores: bool,
}

/// Outcome of checking one [`Intrinsic`] against the local hardware (plan §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicSupport {
    /// The adapter executes the intrinsic natively.
    Native,
    /// No hardware path, but the Forge can lower it to a portable shared-memory
    /// equivalent (e.g. a subgroup reduction → barrier-synchronised tree reduction).
    LowerToSharedMemory,
    /// No hardware and no safe lowering: schedules requiring it must be excluded
    /// from the search on this adapter.
    Exclude,
}

impl HardwareCapabilityMatrix {
    /// Classifies how an intrinsic can be served on this hardware.
    pub const fn intrinsic_support(&self, intrinsic: &Intrinsic) -> IntrinsicSupport {
        match intrinsic.class() {
            IntrinsicClass::Subgroup => {
                if self.subgroup_size.is_some() {
                    IntrinsicSupport::Native
                } else {
                    // Warp reductions/shuffles degrade to a shared-memory tree.
                    IntrinsicSupport::LowerToSharedMemory
                }
            }
            IntrinsicClass::CooperativeMatrix => {
                if self.supports_coopmat {
                    IntrinsicSupport::Native
                } else {
                    IntrinsicSupport::Exclude
                }
            }
            IntrinsicClass::RayTracing => {
                if self.supports_rt_cores {
                    IntrinsicSupport::Native
                } else {
                    IntrinsicSupport::Exclude
                }
            }
        }
    }
}

/// How a 64-bit value would be lowered for a given adapter.
///
/// **Reserved scaffolding (plan §1): not yet exercised by any emitter.** Today every
/// emitter takes the `PairedU32Emulation` shape implicitly via `ScalarType::U64Words`;
/// the `Native` arm has no code path because no IR scalar requests native `f64`/`u64`.
/// Kept so the native-64-bit policy can be selected once such a kernel exists.
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
        Self {
            capabilities,
            schedule,
        }
    }

    /// Selects the 64-bit lowering policy for the current adapter.
    ///
    /// **Reserved scaffolding (plan §1): not yet exercised by any emitter.** No emitter
    /// calls this — 64-bit data flows through `ScalarType::U64Words` (paired-`u32`)
    /// unconditionally. It is retained so a future native-64-bit kernel can branch on
    /// the adapter's `supports_f64` flag without re-introducing the policy from scratch.
    pub fn policy_64bit(&self) -> LoweringPolicy64Bit {
        if self.capabilities.supports_f64 {
            LoweringPolicy64Bit::Native
        } else {
            LoweringPolicy64Bit::PairedU32Emulation
        }
    }

    /// How the local hardware can serve `intrinsic` (native / lower / exclude).
    pub const fn intrinsic_support(&self, intrinsic: &Intrinsic) -> IntrinsicSupport {
        self.capabilities.intrinsic_support(intrinsic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::ir::intrinsics::SubgroupReduceOp;

    fn ray_query() -> Intrinsic {
        Intrinsic::RayQuery {
            acceleration_structure: "tlas".to_string(),
            origin: "o".to_string(),
            direction: "d".to_string(),
            t_min: "tmin".to_string(),
            t_max: "tmax".to_string(),
            destination: "hit".to_string(),
        }
    }

    #[test]
    fn rt_intrinsic_excluded_without_rt_cores() {
        let absent = HardwareCapabilityMatrix::default();
        assert_eq!(
            absent.intrinsic_support(&ray_query()),
            IntrinsicSupport::Exclude
        );

        let present = HardwareCapabilityMatrix {
            supports_rt_cores: true,
            ..Default::default()
        };
        assert_eq!(
            present.intrinsic_support(&ray_query()),
            IntrinsicSupport::Native
        );
    }

    #[test]
    fn coopmat_excluded_but_subgroup_lowers() {
        let caps = HardwareCapabilityMatrix::default();
        assert_eq!(
            caps.intrinsic_support(&Intrinsic::CoopMatMul {
                m: 16,
                n: 16,
                k: 16
            }),
            IntrinsicSupport::Exclude
        );
        // No subgroup hardware → portable shared-memory lowering, not exclusion.
        assert_eq!(
            caps.intrinsic_support(&Intrinsic::SubgroupReduce {
                op: SubgroupReduceOp::Add
            }),
            IntrinsicSupport::LowerToSharedMemory
        );
    }
}
