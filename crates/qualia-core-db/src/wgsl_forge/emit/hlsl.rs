use std::fmt::Write;

use super::{GeneratedShader, TargetBackend};
use crate::wgsl_forge::{ForgeError, KernelSpec, Op, Schedule};

pub fn emit_hlsl(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let semantic_hash = kernel.semantic_hash()?;
    let mut source = String::with_capacity(2048);

    writeln!(source, "// HLSL emitted for {}@{}", kernel.id, kernel.semantic_version)
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
                r#"struct AffineParams {{
    uint length;
    float scale;
    float bias;
    uint _pad;
}};

StructuredBuffer<float> input : register(t0, space0);
RWStructuredBuffer<float> output : register(u1, space0);
ConstantBuffer<AffineParams> params : register(b2, space0);

[numthreads({}, 1, 1)]
void {}(uint3 gid : SV_DispatchThreadID) {{
    const uint ITEMS_PER_INVOCATION = {};
    const uint VECTOR_WIDTH = {};
    
    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{
        uint base = (gid.x * ITEMS_PER_INVOCATION + item) * VECTOR_WIDTH;
        if (base < params.length) {{
            output[base] = input[base] * params.scale + params.bias;
        }}
    }}
}}"#,
                schedule.workgroup_size,
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
