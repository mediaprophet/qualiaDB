use super::{GeneratedShader, TargetBackend};
use crate::wgsl_forge::{ForgeError, KernelSpec, Schedule};

pub(super) fn is_iq(kernel: &KernelSpec) -> bool {
    crate::wgsl_forge::ir::builtin::iq::is_iq(&kernel.id)
}

pub(super) fn emit(
    kernel: &KernelSpec,
    schedule: Schedule,
    target: TargetBackend,
) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let mut limits = crate::wgsl_forge::AdapterConstraints::portable();
    limits.max_workgroup_size_x = 1024;
    limits.max_invocations_per_workgroup = 1024;
    schedule.validate(kernel, &limits)?;
    #[cfg(feature = "gpu-runtime")]
    {
        let canonical: etalon_wgsl_forge::KernelSpec =
            serde_json::from_value(serde_json::to_value(kernel)?)?;
        let generated = etalon_wgsl_forge::emit_shader(&canonical, schedule.into(), target.into())
            .map_err(|error| ForgeError::Emission(error.to_string()))?;
        Ok(generated.into())
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        let _ = target;
        Err(ForgeError::Emission(
            "IQ emission requires the Etalon gpu-runtime dependency".into(),
        ))
    }
}
