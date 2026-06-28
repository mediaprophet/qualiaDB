use std::fmt::Write;

use super::{GeneratedShader, TargetBackend};
use crate::wgsl_forge::{ForgeError, KernelSpec, Op, Schedule};

pub fn emit_ptx(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let semantic_hash = kernel.semantic_hash()?;
    let mut source = String::with_capacity(2048);

    writeln!(source, "// PTX emitted for {}@{}", kernel.id, kernel.semantic_version)
        .map_err(|e| ForgeError::Emission(e.to_string()))?;
    writeln!(source, "// Semantic hash: {}", semantic_hash)
        .map_err(|e| ForgeError::Emission(e.to_string()))?;

    match kernel.ops.first() {
        Some(Op::AffineF32) => {
            writeln!(
                source,
                r#".version 7.5
.target sm_75
.address_size 64

.entry {}(
    .param .u64 input_ptr,
    .param .u64 output_ptr,
    .param .align 4 .b8 params[16]
)
{{
    .reg .pred %p<2>;
    .reg .b32 %r<5>;
    .reg .b64 %rd<5>;
    .reg .f32 %f<5>;

    // global_id = ctaid.x * ntid.x + tid.x
    mov.u32 %r1, %ctaid.x;
    mov.u32 %r2, %ntid.x;
    mov.u32 %r3, %tid.x;
    mad.lo.s32 %r4, %r1, %r2, %r3;

    // Load length from params (offset 0)
    ld.param.u32 %r1, [params+0];
    setp.ge.u32 %p1, %r4, %r1;
    @%p1 bra EXIT;

    // Load addresses
    ld.param.u64 %rd1, [input_ptr];
    ld.param.u64 %rd2, [output_ptr];

    // Load scale (offset 4) and bias (offset 8)
    ld.param.f32 %f1, [params+4];
    ld.param.f32 %f2, [params+8];

    // Calculate memory offset
    mul.wide.u32 %rd3, %r4, 4;
    add.s64 %rd1, %rd1, %rd3;
    add.s64 %rd2, %rd2, %rd3;

    // input[global_id] * scale + bias
    ld.global.f32 %f3, [%rd1];
    fma.rn.f32 %f4, %f3, %f1, %f2;
    st.global.f32 [%rd2], %f4;

EXIT:
    ret;
}}"#,
                kernel.entry_point
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
