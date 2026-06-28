use std::fmt::Write;

use super::{GeneratedShader, TargetBackend};
use crate::wgsl_forge::{ForgeError, KernelSpec, Op, Schedule};

pub fn emit_msl(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let semantic_hash = kernel.semantic_hash()?;
    let mut source = String::with_capacity(2048);

    writeln!(source, "// MSL emitted for {}@{}", kernel.id, kernel.semantic_version)
        .map_err(|e| ForgeError::Emission(e.to_string()))?;
    writeln!(source, "// Semantic hash: {}", semantic_hash)
        .map_err(|e| ForgeError::Emission(e.to_string()))?;
    writeln!(
        source,
        "// Schedule: workgroup={}, items={}, vector={}",
        schedule.workgroup_size, schedule.items_per_invocation, schedule.vector_width
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;

    match kernel.ops.first() {
        Some(Op::AffineF32) => {
            writeln!(
                source,
                r#"#include <metal_stdlib>
using namespace metal;

struct AffineParams {{
    uint length;
    float scale;
    float bias;
    uint _pad;
}};

kernel void {}(
    device const float* input [[buffer(0)]],
    device float* output [[buffer(1)]],
    constant AffineParams& params [[buffer(2)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    const uint ITEMS_PER_INVOCATION = {}u;
    const uint VECTOR_WIDTH = {}u;
    
    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{
        uint base = (gid.x * ITEMS_PER_INVOCATION + item) * VECTOR_WIDTH;
        if (base < params.length) {{
            output[base] = input[base] * params.scale + params.bias;
        }}
    }}
}}"#,
                kernel.entry_point,
                schedule.items_per_invocation,
                schedule.vector_width
            )
            .map_err(|e| ForgeError::Emission(e.to_string()))?;
        }
        _ => return Err(ForgeError::Emission("unsupported operation sequence".to_string())),
    }

    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
    Ok(GeneratedShader {
        kernel_id: kernel.id.clone(),
        semantic_hash,
        source_hash,
        schedule,
        source,
    })
}
