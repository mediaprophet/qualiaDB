use serde::{Deserialize, Serialize};

use super::ir::IntrinsicClass;
use super::{ForgeError, KernelSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Schedule {
    pub workgroup_size: u32,
    pub items_per_invocation: u32,
    pub vector_width: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
        }
    }
}

impl Schedule {
    pub fn validate(
        self,
        kernel: &KernelSpec,
        constraints: &AdapterConstraints,
    ) -> Result<(), ForgeError> {
        kernel.validate()?;
        if !matches!(self.workgroup_size, 1..=1024) || !self.workgroup_size.is_power_of_two() {
            return Err(ForgeError::InvalidSchedule(
                "workgroup size must be a power of two in 1..=1024".to_string(),
            ));
        }
        if self.workgroup_size > constraints.max_workgroup_size_x
            || self.workgroup_size > constraints.max_invocations_per_workgroup
        {
            return Err(ForgeError::InvalidSchedule(format!(
                "workgroup {} exceeds adapter limit {}",
                self.workgroup_size,
                constraints
                    .max_workgroup_size_x
                    .min(constraints.max_invocations_per_workgroup)
            )));
        }
        if !matches!(self.items_per_invocation, 1 | 2 | 4 | 8) {
            return Err(ForgeError::InvalidSchedule(
                "items per invocation must be one of 1, 2, 4, 8".to_string(),
            ));
        }
        if !matches!(self.vector_width, 1 | 2 | 4) {
            return Err(ForgeError::InvalidSchedule(
                "vector width must be one of 1, 2, 4".to_string(),
            ));
        }

        Ok(())
    }

    pub const fn elements_per_workgroup(self) -> u32 {
        self.workgroup_size * self.items_per_invocation * self.vector_width
    }

    pub fn dispatch_workgroups(self, element_count: usize) -> u32 {
        let width = self.elements_per_workgroup().max(1) as usize;
        element_count.div_ceil(width).max(1) as u32
    }

    pub const fn sort_key(self) -> (u32, u32, u32) {
        (
            self.workgroup_size,
            self.items_per_invocation,
            self.vector_width,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterConstraints {
    pub max_workgroup_size_x: u32,
    pub max_invocations_per_workgroup: u32,
    pub max_workgroups_per_dimension: u32,
    pub supports_subgroups: bool,
    /// Cooperative-matrix / Tensor-core matrix-multiply-accumulate support.
    pub supports_coopmat: bool,
    /// Ray-query / RT-core support (hardware ray-triangle intersection).
    pub supports_rt_cores: bool,
    /// SIMD/warp width (32 NVIDIA, 64 AMD/Apple-ish). Workgroup sizes that are
    /// not a multiple of this are pruned from the tuning search (plan §6).
    pub warp_size: u32,
}

impl AdapterConstraints {
    pub const fn portable() -> Self {
        Self {
            max_workgroup_size_x: 256,
            max_invocations_per_workgroup: 256,
            max_workgroups_per_dimension: 65_535,
            supports_subgroups: false,
            supports_coopmat: false,
            supports_rt_cores: false,
            warp_size: 32,
        }
    }

    /// Intrinsic-availability check (plan §6): a kernel that requires RT or
    /// cooperative-matrix hardware cannot run on an adapter that lacks it, so such
    /// kernels are pruned before tuning/certification on the local adapter.
    /// Subgroup intrinsics are not pruned because they can be lowered to a
    /// shared-memory equivalent. This is a hardware-vs-kernel check and is
    /// deliberately separate from schedule emission (which is hardware-agnostic).
    pub fn supports_kernel(&self, kernel: &KernelSpec) -> Result<(), ForgeError> {
        for intrinsic in kernel.required_intrinsics() {
            match intrinsic.class() {
                IntrinsicClass::CooperativeMatrix if !self.supports_coopmat => {
                    return Err(ForgeError::InvalidSchedule(
                        "kernel requires cooperative-matrix (tensor cores) unavailable on this adapter".to_string(),
                    ));
                }
                IntrinsicClass::RayTracing if !self.supports_rt_cores => {
                    return Err(ForgeError::InvalidSchedule(
                        "kernel requires ray-query (RT cores) unavailable on this adapter".to_string(),
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[cfg(feature = "gpu-runtime")]
    pub fn from_wgpu_limits(limits: &wgpu::Limits) -> Self {
        // Intrinsic-capability flags default to false here and are populated from
        // the adapter's feature set at device creation (see WgpuComputeContext).
        Self {
            max_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            supports_subgroups: false,
            supports_coopmat: false,
            supports_rt_cores: false,
            warp_size: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleSpace {
    pub workgroup_sizes: Vec<u32>,
    pub items_per_invocation: Vec<u32>,
    pub vector_widths: Vec<u32>,
}

impl Default for ScheduleSpace {
    fn default() -> Self {
        Self {
            workgroup_sizes: vec![32, 64, 128, 256],
            items_per_invocation: vec![1, 2, 4, 8],
            vector_widths: vec![1, 2, 4],
        }
    }
}

impl ScheduleSpace {
    /// Generate the pruned candidate schedules for `kernel` on this adapter.
    ///
    /// Pruning here is limited to what is portably knowable: adapter workgroup/
    /// invocation limits and `Schedule::validate`, plus warp-size alignment.
    ///
    /// **Known limitations (plan §6, honest — not implemented):** candidates are NOT
    /// pruned by a device-relative roofline (wgpu exposes no peak FLOPS/bandwidth — see
    /// [`crate::wgsl_forge::roofline`]), by compute-unit saturation (no CU/SM count is
    /// exposed by wgpu), or by a cross-vendor thermal/power model (no portable
    /// temperature sensor; thermal pacing is handled NVIDIA-only via `nvidia-smi` in the
    /// `auto-tune-all` CLI loop, not here). Any of these would need a calibration
    /// micro-benchmark or a platform-specific probe.
    pub fn candidates(
        &self,
        kernel: &KernelSpec,
        constraints: &AdapterConstraints,
    ) -> Vec<Schedule> {
        let mut candidates = Vec::new();
        for &workgroup_size in &self.workgroup_sizes {
            for &items_per_invocation in &self.items_per_invocation {
                for &vector_width in &self.vector_widths {
                    let schedule = Schedule {
                        workgroup_size,
                        items_per_invocation,
                        vector_width,
                        ..Default::default()
                    };
                    // Prune non-warp-aligned workgroups (plan §6): a partial warp
                    // wastes lanes. Generation is unaffected — this only narrows
                    // the tuning search.
                    let warp_aligned = constraints.warp_size <= 1
                        || workgroup_size % constraints.warp_size == 0;
                    if warp_aligned && schedule.validate(kernel, constraints).is_ok() {
                        candidates.push(schedule);
                    }
                }
            }
        }
        candidates.sort_by_key(|schedule| schedule.sort_key());
        candidates.dedup();
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::BuiltinKernel;

    #[test]
    fn candidates_are_bounded_and_deterministic() {
        let kernel = BuiltinKernel::AffineF32.spec();
        let constraints = AdapterConstraints::portable();
        let first = ScheduleSpace::default().candidates(&kernel, &constraints);
        let second = ScheduleSpace::default().candidates(&kernel, &constraints);
        assert_eq!(first, second);
        assert_eq!(first.len(), 48);
        assert!(first.iter().all(|schedule| schedule.workgroup_size <= 256));
    }

    #[test]
    fn dispatch_covers_tail_elements() {
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 2,
            vector_width: 4,
            ..Default::default()
        };
        assert_eq!(schedule.dispatch_workgroups(513), 2);
    }

    #[test]
    fn invalid_schedule_is_rejected_before_emission() {
        use crate::wgsl_forge::{generate_builtin, ForgeError, TargetBackend};

        // workgroup_size = 100 is not a power of two, so `Schedule::validate`
        // (invoked by `generate_builtin` *before* `emit_shader`) must reject it.
        let invalid = Schedule {
            workgroup_size: 100,
            items_per_invocation: 1,
            vector_width: 1,
        };
        // Guard the precondition: the schedule itself is invalid.
        let kernel = BuiltinKernel::AffineF32.spec();
        assert!(invalid.validate(&kernel, &AdapterConstraints::portable()).is_err());

        let result = generate_builtin(BuiltinKernel::AffineF32, invalid, TargetBackend::Wgsl);
        // It must be the schedule-validation error, not an emission error — proving
        // generation never ran for an invalid schedule.
        match result {
            Err(ForgeError::InvalidSchedule(message)) => {
                assert!(
                    message.contains("power of two"),
                    "expected a power-of-two schedule error, got: {message}"
                );
            }
            other => panic!("expected InvalidSchedule before emission, got {other:?}"),
        }
    }

    #[test]
    fn rt_kernel_pruned_without_rt_cores() {
        use crate::wgsl_forge::ir::{
            BufferAccess, BufferElement, BufferSpec, Intrinsic, KernelSpec, Op, ScalarType,
        };

        let kernel = KernelSpec {
            id: "rt-probe".to_string(),
            semantic_version: 1,
            entry_point: "rt_probe".to_string(),
            description: "ray-query smoke kernel".to_string(),
            buffers: vec![BufferSpec {
                group: 0,
                binding: 0,
                name: "output".to_string(),
                element: BufferElement::Scalar(ScalarType::F32),
                access: BufferAccess::StorageReadWrite,
            }],
            ops: vec![Op::Intrinsic(Intrinsic::RayQuery {
                acceleration_structure: "tlas".to_string(),
                origin: "o".to_string(),
                direction: "d".to_string(),
                t_min: "tmin".to_string(),
                t_max: "tmax".to_string(),
                destination: "hit".to_string(),
            })],
            shared_memory: Vec::new(),
        };

        // Generation/schedule validity is hardware-agnostic and must succeed.
        assert!(Schedule::default().validate(&kernel, &AdapterConstraints::portable()).is_ok());

        // The hardware-vs-kernel check prunes it when RT cores are absent...
        let without = AdapterConstraints::portable();
        assert!(without.supports_kernel(&kernel).is_err());

        // ...and accepts it when present.
        let with = AdapterConstraints {
            supports_rt_cores: true,
            ..AdapterConstraints::portable()
        };
        assert!(with.supports_kernel(&kernel).is_ok());
    }
}
