use serde::{Deserialize, Serialize};

use super::{ForgeError, KernelSpec, Op};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Schedule {
    pub workgroup_size: u32,
    pub items_per_invocation: u32,
    pub vector_width: u32,
    pub tile_mnk: Option<[u32; 3]>,
    pub use_subgroup: bool,
    pub prefetch: bool,
    pub unroll_factor: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            tile_mnk: None,
            use_subgroup: false,
            prefetch: false,
            unroll_factor: 1,
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
        if self.use_subgroup && !constraints.supports_subgroups {
            return Err(ForgeError::InvalidSchedule("adapter does not support subgroups".to_string()));
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
    pub supports_coopmat: bool,
}

impl AdapterConstraints {
    pub const fn portable() -> Self {
        Self {
            max_workgroup_size_x: 256,
            max_invocations_per_workgroup: 256,
            max_workgroups_per_dimension: 65_535,
            supports_subgroups: false,
            supports_coopmat: false,
        }
    }

    #[cfg(feature = "gpu-runtime")]
    pub fn from_wgpu_limits(limits: &wgpu::Limits) -> Self {
        Self {
            max_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            supports_subgroups: false, // Discovered at device creation
            supports_coopmat: false,
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
                    if schedule.validate(kernel, constraints).is_ok() {
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
}
