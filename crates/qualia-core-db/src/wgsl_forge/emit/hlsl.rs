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

    emit_kernel_body(&mut source, kernel, schedule)?;

    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
    Ok(GeneratedShader {
        kernel_id: kernel.id.clone(),
        semantic_hash,
        source_hash,
        schedule,
        source,
    })
}

fn emit_kernel_body(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    if kernel.id == "topk" {
        return emit_topk_hlsl(source, kernel, schedule);
    }
    if kernel.id == "fused-ffn" {
        return emit_ffn_hlsl(source, kernel, schedule);
    }
    if kernel.id == "p64-project" {
        return emit_p64_hlsl(source, kernel, schedule);
    }
    if kernel.id == "ray-probe" {
        return Err(ForgeError::Emission(
            "ray-query is only emitted for the WGSL target (HLSL RT uses a distinct API)".to_string(),
        ));
    }
    if kernel.id == "affine-f32" {
        writeln!(
            source,
            r#"struct AffineParams {{
    uint length;
    float scale;
    float bias;
    uint _pad;
}};"#
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    if kernel.id == "p64-project" {
        writeln!(
            source,
            r#"struct P64Words64 {{
    uint4 lanes[4];
}};"#
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    writeln!(source, "").map_err(|error| ForgeError::Emission(error.to_string()))?;
    
    for buffer in &kernel.buffers {
        let (type_decl, reg_type) = match (buffer.element, buffer.access) {
            (crate::wgsl_forge::ir::BufferElement::AffineParams, _) => ("ConstantBuffer<AffineParams>", "b"),
            (crate::wgsl_forge::ir::BufferElement::P64Words64, crate::wgsl_forge::ir::BufferAccess::StorageRead) => ("StructuredBuffer<P64Words64>", "t"),
            (crate::wgsl_forge::ir::BufferElement::P64Words64, crate::wgsl_forge::ir::BufferAccess::StorageReadWrite) => ("RWStructuredBuffer<P64Words64>", "u"),
            (crate::wgsl_forge::ir::BufferElement::Scalar(crate::wgsl_forge::ir::ScalarType::F32), crate::wgsl_forge::ir::BufferAccess::StorageRead) => ("StructuredBuffer<float>", "t"),
            (crate::wgsl_forge::ir::BufferElement::Scalar(crate::wgsl_forge::ir::ScalarType::F32), crate::wgsl_forge::ir::BufferAccess::StorageReadWrite) => ("RWStructuredBuffer<float>", "u"),
            _ => ("StructuredBuffer<float>", "t") // Fallback
        };
        writeln!(
            source,
            "{} {} : register({}{}, space{});",
            type_decl, buffer.name, reg_type, buffer.binding, buffer.group
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    writeln!(source, "\n[numthreads({}, 1, 1)]", schedule.workgroup_size).map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "void {}(uint3 gid : SV_DispatchThreadID) {{", kernel.entry_point).map_err(|error| ForgeError::Emission(error.to_string()))?;

    writeln!(
        source,
        "    const uint ITEMS_PER_INVOCATION = {};\n    const uint VECTOR_WIDTH = {};",
        schedule.items_per_invocation,
        schedule.vector_width
    ).map_err(|error| ForgeError::Emission(error.to_string()))?;

    if kernel.id == "affine-f32" {
        writeln!(
            source,
            "    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{"
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
        writeln!(
            source,
            "        uint global_id = (gid.x * ITEMS_PER_INVOCATION + item) * VECTOR_WIDTH;"
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
        
        if schedule.vector_width == 1 {
            writeln!(source, "        if (global_id < params.length) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
            emit_ops(source, &kernel.ops, "            ")?;
            writeln!(source, "        }}").map_err(|error| ForgeError::Emission(error.to_string()))?;
        } else {
            // Simplified for now, fallback to loop for vector sizes in HLSL
            writeln!(source, "        if (global_id + {} < params.length) {{", schedule.vector_width - 1).map_err(|error| ForgeError::Emission(error.to_string()))?;
            for index in 0..schedule.vector_width {
                writeln!(source, "            output[global_id + {index}] = input[global_id + {index}] * params.scale + params.bias;").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            writeln!(source, "        }} else {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "            for (uint component = 0; component < VECTOR_WIDTH; component++) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                uint base = global_id + component;").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                if (base < params.length) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                    output[base] = input[base] * params.scale + params.bias;").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                }}").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "            }}").map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "        }}").map_err(|error| ForgeError::Emission(error.to_string()))?;
        }
        writeln!(source, "    }}\n}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
    } else {
        writeln!(
            source,
            "    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{"
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
        writeln!(
            source,
            "        uint global_id = gid.x * ITEMS_PER_INVOCATION + item;"
        ).map_err(|error| ForgeError::Emission(error.to_string()))?;
        emit_ops(source, &kernel.ops, "        ")?;
        writeln!(source, "    }}\n}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    Ok(())
}

/// Top-k reduction in HLSL (compute shader 6.0): one thread group per block,
/// `k` largest values per block in descending order, using `groupshared` arrays
/// (driven by the IR) and `GroupMemoryBarrierWithGroupSync`.
fn emit_topk_hlsl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let wg = schedule.workgroup_size;
    writeln!(
        source,
        "struct TopKParams {{\n    uint length;\n    uint k;\n    uint block_size;\n    uint _pad;\n}};\n"
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;

    writeln!(source, "StructuredBuffer<float> input : register(t0, space0);")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "RWStructuredBuffer<float> output : register(u1, space0);")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "ConstantBuffer<TopKParams> params : register(b2, space0);\n")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;

    for shared in &kernel.shared_memory {
        let ty = hlsl_scalar(shared.element);
        writeln!(
            source,
            "groupshared {} {}[{}];",
            ty,
            shared.name,
            shared.length.resolve(wg)
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    writeln!(
        source,
        r#"
[numthreads({wg}, 1, 1)]
void {entry}(uint tid : SV_GroupIndex, uint3 group_id : SV_GroupID) {{
    uint block = group_id.x;
    uint base = block * {wg}u;
    uint gidx = base + tid;
    float sentinel = asfloat(0xff7fffffu);
    float v = sentinel;
    if (gidx < params.length) {{ v = input[gidx]; }}
    s_val[tid] = v;
    s_idx[tid] = tid;
    GroupMemoryBarrierWithGroupSync();

    for (uint i = 0u; i < params.k; i++) {{
        r_val[tid] = s_val[tid];
        r_idx[tid] = s_idx[tid];
        GroupMemoryBarrierWithGroupSync();
        for (uint stride = {wg}u / 2u; stride > 0u; stride /= 2u) {{
            if (tid < stride) {{
                if (r_val[tid + stride] > r_val[tid]) {{
                    r_val[tid] = r_val[tid + stride];
                    r_idx[tid] = r_idx[tid + stride];
                }}
            }}
            GroupMemoryBarrierWithGroupSync();
        }}
        if (tid == 0u) {{
            output[block * params.k + i] = r_val[0];
            s_val[r_idx[0]] = sentinel;
        }}
        GroupMemoryBarrierWithGroupSync();
    }}
}}"#,
        wg = wg,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;

    Ok(())
}

/// Fused FFN in HLSL (cs_6_0): one thread per output element (see the WGSL
/// emitter for the math).
fn emit_ffn_hlsl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let wg = schedule.workgroup_size;
    writeln!(
        source,
        r#"struct FfnParams {{
    uint input_size;
    uint hidden_size;
    uint output_size;
    uint _pad;
}};

StructuredBuffer<float> input : register(t0, space0);
StructuredBuffer<float> w1 : register(t1, space0);
StructuredBuffer<float> w2 : register(t2, space0);
RWStructuredBuffer<float> output : register(u3, space0);
ConstantBuffer<FfnParams> params : register(b4, space0);

[numthreads({wg}, 1, 1)]
void {entry}(uint3 gid : SV_DispatchThreadID) {{
    uint o = gid.x;
    if (o >= params.output_size) {{ return; }}
    float acc = 0.0;
    for (uint h = 0; h < params.hidden_size; h++) {{
        float hv = 0.0;
        uint w1_row = h * params.input_size;
        for (uint i = 0; i < params.input_size; i++) {{ hv += w1[w1_row + i] * input[i]; }}
        float g = 0.5f * hv * (1.0f + tanh(0.7978845608f * (hv + 0.044715f * hv * hv * hv)));
        acc += w2[o * params.hidden_size + h] * g;
    }}
    output[o] = acc;
}}"#,
        wg = wg,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// P64 descriptor projection in HLSL: one thread per record, bound via the
/// structured buffer's GetDimensions (no length uniform needed).
fn emit_p64_hlsl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let wg = schedule.workgroup_size;
    writeln!(
        source,
        r#"struct P64Words64 {{
    uint4 lanes[4];
}};

StructuredBuffer<P64Words64> input : register(t0, space0);
StructuredBuffer<float> weights : register(t1, space0);
RWStructuredBuffer<float> output : register(u2, space0);

[numthreads({wg}, 1, 1)]
void {entry}(uint3 gid : SV_DispatchThreadID) {{
    uint r = gid.x;
    uint count, stride;
    output.GetDimensions(count, stride);
    if (r >= count) {{ return; }}
    P64Words64 rec = input[r];
    float acc = 0.0;
    for (uint w = 0; w < 16; w++) {{
        uint word = rec.lanes[w / 4][w % 4];
        acc += weights[w] * (float)word;
    }}
    output[r] = acc;
}}"#,
        wg = wg,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

fn hlsl_scalar(element: crate::wgsl_forge::ir::ScalarType) -> &'static str {
    use crate::wgsl_forge::ir::ScalarType;
    match element {
        ScalarType::F32 => "float",
        ScalarType::U32 => "uint",
        ScalarType::I32 => "int",
        ScalarType::U64Words => "uint2",
    }
}

fn emit_ops(source: &mut String, ops: &[Op], indent: &str) -> Result<(), ForgeError> {
    for op in ops {
        match op {
            Op::StructLoad { buffer, field, destination } => {
                writeln!(source, "{indent}float {destination} = {buffer}.{field};").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Load { buffer, index, destination } => {
                writeln!(source, "{indent}float {destination} = {buffer}[{index}];").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Store { buffer, index, value } => {
                writeln!(source, "{indent}{buffer}[{index}] = {value};").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Fma { a, b, c, destination } => {
                writeln!(source, "{indent}float {destination} = {a} * {b} + {c};").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Mul { left, right, destination } => {
                writeln!(source, "{indent}float {destination} = {left} * {right};").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Add { left, right, destination } => {
                writeln!(source, "{indent}float {destination} = {left} + {right};").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::DotProduct { left_buffer, left_base, right_buffer, right_base, len, destination } => {
                writeln!(source, "{indent}float {destination} = 0.0;").map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}for (uint i = 0; i < {len}; i++) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}    {destination} += {left_buffer}[{left_base} + i] * {right_buffer}[{right_base} + i];").map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Loop { induction_var, start, end, step, body } => {
                writeln!(source, "{indent}for (uint {induction_var} = {start}; {induction_var} < {end}; {induction_var} += {step}) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
                emit_ops(source, body, &format!("{indent}    "))?;
                writeln!(source, "{indent}}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Relu { operand, destination } => {
                writeln!(source, "{indent}float {destination} = max(0.0f, {operand});").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Gelu { operand, destination } => {
                writeln!(source, "{indent}float {destination} = 0.5f * {operand} * (1.0f + tanh(0.7978845608f * ({operand} + 0.044715f * {operand} * {operand} * {operand})));").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::MatrixMultiply { left_buffer, right_buffer, destination, m, n, k } => {
                writeln!(source, "{indent}// MatrixMultiply intrinsic placeholder").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Barrier => {
                writeln!(source, "{indent}GroupMemoryBarrierWithGroupSync();").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Intrinsic(_) => {
                return Err(ForgeError::Emission("Intrinsics not implemented for HLSL yet".to_string()));
            }
        }
    }
    Ok(())
}
