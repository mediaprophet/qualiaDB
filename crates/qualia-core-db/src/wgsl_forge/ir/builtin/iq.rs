use super::super::core::{BufferAccess, BufferElement, BufferSpec, KernelSpec, ScalarType};
use crate::wgsl_forge::ForgeError;

pub(crate) fn is_iq(id: &str) -> bool {
    matches!(id, "iq-power-f32" | "iq-power-i16")
}

pub(super) fn spec(packed_i16: bool) -> KernelSpec {
    let (id, entry_point, description, input, params) = if packed_i16 {
        (
            "iq-power-i16",
            "iq_power_i16",
            "Decode packed signed-i16 IQ, scale in f32, and sum squared components",
            ScalarType::U32,
            BufferElement::IqPowerI16Params,
        )
    } else {
        (
            "iq-power-f32",
            "iq_power_f32",
            "out[i] = input[2*i]^2 + input[2*i+1]^2",
            ScalarType::F32,
            BufferElement::IqPowerParams,
        )
    };
    KernelSpec {
        id: id.into(),
        semantic_version: 1,
        entry_point: entry_point.into(),
        description: description.into(),
        buffers: [
            (
                "input",
                BufferElement::Scalar(input),
                BufferAccess::StorageRead,
            ),
            (
                "output",
                BufferElement::Scalar(ScalarType::F32),
                BufferAccess::StorageReadWrite,
            ),
            ("params", params, BufferAccess::Uniform),
        ]
        .into_iter()
        .enumerate()
        .map(|(binding, (name, element, access))| BufferSpec {
            group: 0,
            binding: binding as u32,
            name: name.into(),
            element,
            access,
        })
        .collect(),
        ops: Vec::new(),
        shared_memory: Vec::new(),
    }
}

pub(crate) fn validate(kernel: &KernelSpec) -> Result<(), ForgeError> {
    if is_iq(&kernel.id) {
        if *kernel != spec(kernel.id == "iq-power-i16") {
            return Err(ForgeError::InvalidKernel(
                "IQ kernel must retain its canonical metadata, bindings and operation contract"
                    .into(),
            ));
        }
    } else if kernel.buffers.iter().any(|buffer| {
        matches!(
            buffer.element,
            BufferElement::IqPowerParams | BufferElement::IqPowerI16Params
        )
    }) {
        return Err(ForgeError::InvalidKernel(
            "IQ parameter records require a canonical IQ kernel".into(),
        ));
    }
    Ok(())
}
