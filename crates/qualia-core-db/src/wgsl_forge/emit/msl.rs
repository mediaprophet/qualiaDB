use std::fmt::Write;

use super::GeneratedShader;
use crate::wgsl_forge::{ForgeError, KernelSpec, Op, Schedule};

pub fn emit_msl(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    kernel.validate()?;
    let semantic_hash = kernel.semantic_hash()?;
    let mut source = String::with_capacity(2048);

    writeln!(
        source,
        "// MSL emitted for {}@{}",
        kernel.id, kernel.semantic_version
    )
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
        return emit_topk_msl(source, kernel, schedule);
    }
    if kernel.id == "fused-ffn" {
        return emit_ffn_msl(source, kernel, schedule);
    }
    if kernel.id == "p64-project" {
        return emit_p64_msl(source, kernel, schedule);
    }
    if kernel.id == "gemm" {
        return emit_gemm_msl(source, kernel, schedule);
    }
    if kernel.id == "gemv" {
        return emit_gemv_msl(source, kernel, schedule);
    }
    if kernel.id == "ternary-gemv" {
        return emit_ternary_gemv_msl(source, kernel, schedule);
    }
    if kernel.id == "fft" {
        return emit_fft_msl(source, kernel, schedule);
    }
    if kernel.id == "ray-probe" {
        return Err(ForgeError::Emission(
            "ray-query is only emitted for the WGSL target (Metal RT uses a distinct API)"
                .to_string(),
        ));
    }
    writeln!(source, "#include <metal_stdlib>\nusing namespace metal;\n")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;

    if kernel.id == "affine-f32" {
        writeln!(
            source,
            r#"struct AffineParams {{
    uint length;
    float scale;
    float bias;
    uint _pad;
}};"#
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    }
    writeln!(source, "").map_err(|error| ForgeError::Emission(error.to_string()))?;

    writeln!(source, "kernel void {}(", kernel.entry_point)
        .map_err(|error| ForgeError::Emission(error.to_string()))?;

    for (i, buffer) in kernel.buffers.iter().enumerate() {
        let type_decl = match (buffer.element, buffer.access) {
            (crate::wgsl_forge::ir::BufferElement::AffineParams, _) => "constant AffineParams&",
            (
                crate::wgsl_forge::ir::BufferElement::P64Words64,
                crate::wgsl_forge::ir::BufferAccess::StorageRead,
            ) => "device const P64Words64*",
            (
                crate::wgsl_forge::ir::BufferElement::P64Words64,
                crate::wgsl_forge::ir::BufferAccess::StorageReadWrite,
            ) => "device P64Words64*",
            (
                crate::wgsl_forge::ir::BufferElement::Scalar(
                    crate::wgsl_forge::ir::ScalarType::F32,
                ),
                crate::wgsl_forge::ir::BufferAccess::StorageRead,
            ) => "device const float*",
            (
                crate::wgsl_forge::ir::BufferElement::Scalar(
                    crate::wgsl_forge::ir::ScalarType::F32,
                ),
                crate::wgsl_forge::ir::BufferAccess::StorageReadWrite,
            ) => "device float*",
            _ => "device float*",
        };
        let separator = if i < kernel.buffers.len() - 1 {
            ","
        } else {
            ""
        };
        writeln!(
            source,
            "    {} {} [[buffer({})]]{}",
            type_decl, buffer.name, buffer.binding, separator
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    }
    writeln!(source, "    , uint3 gid [[thread_position_in_grid]]")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, ") {{").map_err(|error| ForgeError::Emission(error.to_string()))?;

    writeln!(
        source,
        "    const uint ITEMS_PER_INVOCATION = {}u;\n    const uint VECTOR_WIDTH = {}u;",
        schedule.items_per_invocation, schedule.vector_width
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;

    if kernel.id == "affine-f32" {
        writeln!(
            source,
            "    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{"
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
        writeln!(
            source,
            "        uint global_id = (gid.x * ITEMS_PER_INVOCATION + item) * VECTOR_WIDTH;"
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;

        if schedule.vector_width == 1 {
            writeln!(source, "        if (global_id < params.length) {{")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            emit_ops(source, &kernel.ops, "            ")?;
            writeln!(source, "        }}")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
        } else {
            // Vectorized affine (affine-f32 is the only kernel that sets vector_width>1):
            // an unrolled fast path when the whole VECTOR_WIDTH span is in bounds, plus a
            // bounds-checked tail for the final partial span. Correct for affine-f32 (its sole
            // op is out = in*scale + bias). Native Metal float4 SIMD loads would be a throughput
            // optimization (needs a Metal compiler to validate — absent on this host), not a gap.
            writeln!(
                source,
                "        if (global_id + {}u < params.length) {{",
                schedule.vector_width - 1
            )
            .map_err(|error| ForgeError::Emission(error.to_string()))?;
            for index in 0..schedule.vector_width {
                writeln!(source, "            output[global_id + {index}u] = input[global_id + {index}u] * params.scale + params.bias;").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            writeln!(source, "        }} else {{")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(
                source,
                "            for (uint component = 0; component < VECTOR_WIDTH; component++) {{"
            )
            .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                uint base = global_id + component;")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                if (base < params.length) {{")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(
                source,
                "                    output[base] = input[base] * params.scale + params.bias;"
            )
            .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "                }}")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "            }}")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            writeln!(source, "        }}")
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
        }
        writeln!(source, "    }}\n}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
    } else {
        writeln!(
            source,
            "    for (uint item = 0; item < ITEMS_PER_INVOCATION; item++) {{"
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
        writeln!(
            source,
            "        uint global_id = gid.x * ITEMS_PER_INVOCATION + item;"
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
        emit_ops(source, &kernel.ops, "        ")?;
        writeln!(source, "    }}\n}}").map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    Ok(())
}

/// Top-k reduction in Metal: one threadgroup per block, `k` largest values per
/// block in descending order, using `threadgroup` shared arrays (driven by the
/// IR) and `threadgroup_barrier`.
fn emit_topk_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let wg = schedule.workgroup_size;
    writeln!(source, "#include <metal_stdlib>\nusing namespace metal;\n")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(
        source,
        "struct TopKParams {{\n    uint length;\n    uint k;\n    uint block_size;\n    uint _pad;\n}};\n"
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;

    writeln!(source, "kernel void {}(", kernel.entry_point)
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "    device const float* input [[buffer(0)]],")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "    device float* output [[buffer(1)]],")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "    constant TopKParams& params [[buffer(2)]],")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "    uint tid [[thread_position_in_threadgroup]],")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, "    uint block [[threadgroup_position_in_grid]]")
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    writeln!(source, ") {{").map_err(|error| ForgeError::Emission(error.to_string()))?;

    for shared in &kernel.shared_memory {
        let ty = msl_scalar(shared.element);
        writeln!(
            source,
            "    threadgroup {} {}[{}];",
            ty,
            shared.name,
            shared.length.resolve(wg)
        )
        .map_err(|error| ForgeError::Emission(error.to_string()))?;
    }

    writeln!(
        source,
        r#"
    uint base = block * {wg}u;
    uint gidx = base + tid;
    float sentinel = as_type<float>(0xff7fffffu);
    float v = sentinel;
    if (gidx < params.length) {{ v = input[gidx]; }}
    s_val[tid] = v;
    s_idx[tid] = tid;
    threadgroup_barrier(mem_flags::mem_threadgroup);

    for (uint i = 0u; i < params.k; i++) {{
        r_val[tid] = s_val[tid];
        r_idx[tid] = s_idx[tid];
        threadgroup_barrier(mem_flags::mem_threadgroup);
        for (uint stride = {wg}u / 2u; stride > 0u; stride /= 2u) {{
            if (tid < stride) {{
                if (r_val[tid + stride] > r_val[tid]) {{
                    r_val[tid] = r_val[tid + stride];
                    r_idx[tid] = r_idx[tid + stride];
                }}
            }}
            threadgroup_barrier(mem_flags::mem_threadgroup);
        }}
        if (tid == 0u) {{
            output[block * params.k + i] = r_val[0];
            s_val[r_idx[0]] = sentinel;
        }}
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }}
}}"#,
        wg = wg
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;

    Ok(())
}

/// Fused FFN in Metal: one thread per output element (see the WGSL emitter for
/// the math). Self-contained nested matvec + GELU + accumulate.
fn emit_ffn_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let _ = schedule;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct FfnParams {{
    uint input_size;
    uint hidden_size;
    uint output_size;
    uint _pad;
}};

kernel void {entry}(
    device const float* input [[buffer(0)]],
    device const float* w1 [[buffer(1)]],
    device const float* w2 [[buffer(2)]],
    device float* output [[buffer(3)]],
    constant FfnParams& params [[buffer(4)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint o = gid.x;
    if (o >= params.output_size) {{ return; }}
    float acc = 0.0f;
    for (uint h = 0; h < params.hidden_size; h++) {{
        float hv = 0.0f;
        uint w1_row = h * params.input_size;
        for (uint i = 0; i < params.input_size; i++) {{ hv += w1[w1_row + i] * input[i]; }}
        float g = 0.5f * hv * (1.0f + tanh(0.7978845608f * (hv + 0.044715f * hv * hv * hv)));
        acc += w2[o * params.hidden_size + h] * g;
    }}
    output[o] = acc;
}}"#,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// P64 descriptor projection in Metal: one thread per record. Metal device buffers
/// carry no length, so the record count travels in a `P64Params` constant (the WGSL
/// kernel uses `arrayLength`, HLSL uses `GetDimensions`) — same math, same bindings.
fn emit_p64_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let _ = schedule;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct P64Words64 {{
    uint4 lanes[4];
}};

struct P64Params {{
    uint record_count;
    uint _pad0;
    uint _pad1;
    uint _pad2;
}};

kernel void {entry}(
    device const P64Words64* input [[buffer(0)]],
    device const float* weights [[buffer(1)]],
    device float* output [[buffer(2)]],
    constant P64Params& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint r = gid.x;
    if (r >= params.record_count) {{ return; }}
    P64Words64 rec = input[r];
    float acc = 0.0f;
    for (uint w = 0; w < 16u; w++) {{
        uint word = rec.lanes[w / 4u][w % 4u];
        acc += weights[w] * (float)word;
    }}
    output[r] = acc;
}}"#,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// Dense row-major GEMM in Metal: one thread per output element, same binding order,
/// params layout and accumulation order as the certified WGSL `gemm`.
fn emit_gemm_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let _ = schedule;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct GemmParams {{
    uint m;
    uint n;
    uint k;
    uint _pad;
}};

kernel void {entry}(
    device const float* a [[buffer(0)]],
    device const float* b [[buffer(1)]],
    device float* c [[buffer(2)]],
    constant GemmParams& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint o = gid.x;
    if (o >= params.m * params.n) {{ return; }}
    uint row = o / params.n;
    uint col = o % params.n;
    float acc = 0.0f;
    uint a_row = row * params.k;
    for (uint kk = 0; kk < params.k; kk++) {{
        acc += a[a_row + kk] * b[kk * params.n + col];
    }}
    c[o] = acc;
}}"#,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// Dense row-major GEMV in Metal: one thread per output ROW, same order as WGSL `gemv`.
fn emit_gemv_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let _ = schedule;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct GemvParams {{
    uint m;
    uint n;
    uint _pad0;
    uint _pad1;
}};

kernel void {entry}(
    device const float* a [[buffer(0)]],
    device const float* x [[buffer(1)]],
    device float* y [[buffer(2)]],
    constant GemvParams& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint i = gid.x;
    if (i >= params.m) {{ return; }}
    float acc = 0.0f;
    uint a_row = i * params.n;
    for (uint j = 0; j < params.n; j++) {{
        acc += a[a_row + j] * x[j];
    }}
    y[i] = acc;
}}"#,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// BitNet-style ternary GEMV in Metal: one thread per output row. 2-bit codes,
/// 16 per `uint` (`0->0.0, 1->+1.0, 2->-1.0, 3->0.0`), `k_words` per row. `w_packed`
/// is `device const uint*` — the generic path wrongly typed it float.
fn emit_ternary_gemv_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let _ = schedule;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct TernaryGemvParams {{
    uint m;
    uint k;
    uint k_words;
    uint _pad;
}};

kernel void {entry}(
    device const float* x [[buffer(0)]],
    device const uint* w_packed [[buffer(1)]],
    device const float* scale [[buffer(2)]],
    device float* output [[buffer(3)]],
    constant TernaryGemvParams& params [[buffer(4)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint o = gid.x;
    if (o >= params.m) {{ return; }}
    float acc = 0.0f;
    uint row_base = o * params.k_words;
    for (uint word_idx = 0; word_idx < params.k_words; word_idx++) {{
        uint word = w_packed[row_base + word_idx];
        uint lane_base = word_idx * 16u;
        for (uint lane = 0; lane < 16u; lane++) {{
            uint i = lane_base + lane;
            if (i >= params.k) {{ break; }}
            uint code = (word >> (lane * 2u)) & 3u;
            float tern = 0.0f;
            if (code == 1u) {{ tern = 1.0f; }} else if (code == 2u) {{ tern = -1.0f; }}
            acc += tern * x[i];
        }}
    }}
    output[o] = scale[o] * acc;
}}"#,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

/// Forward radix-2 DIT FFT in Metal over ONE threadgroup of `N = workgroup_size`
/// threads. Interleaved complex f32, bit-reversal load into `threadgroup` arrays,
/// then `log2(N)` butterfly stages with `threadgroup_barrier`. Same
/// `exp(-2*pi*i*k/m)` convention as the WGSL kernel and the CPU DFT oracle;
/// `reverse_bits` is the Metal intrinsic (WGSL spells it `reverseBits`).
fn emit_fft_msl(
    source: &mut String,
    kernel: &KernelSpec,
    schedule: Schedule,
) -> Result<(), ForgeError> {
    let wg = schedule.workgroup_size;
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct FftParams {{
    uint n;
    uint log2n;
    uint _pad0;
    uint _pad1;
}};

kernel void {entry}(
    device const float* input [[buffer(0)]],
    device float* output [[buffer(1)]],
    constant FftParams& params [[buffer(2)]],
    uint tid [[thread_position_in_threadgroup]]
) {{
    threadgroup float s_re[{wg}];
    threadgroup float s_im[{wg}];
    uint t = tid;
    uint n = params.n;
    uint logn = params.log2n;
    uint rev = reverse_bits(t) >> (32u - logn);
    s_re[rev] = input[2u * t];
    s_im[rev] = input[2u * t + 1u];
    threadgroup_barrier(mem_flags::mem_threadgroup);
    for (uint s = 0u; s < logn; s++) {{
        uint span = 1u << s;
        uint m = span << 1u;
        if (t < (n >> 1u)) {{
            uint k = t & (span - 1u);
            uint j = ((t >> s) << (s + 1u)) + k;
            uint jp = j + span;
            float ang = -6.28318548f * (float)k / (float)m;
            float wr = cos(ang);
            float wi = sin(ang);
            float ur = s_re[j];
            float ui = s_im[j];
            float vr = s_re[jp];
            float vi = s_im[jp];
            float tr = vr * wr - vi * wi;
            float ti = vr * wi + vi * wr;
            s_re[j] = ur + tr;
            s_im[j] = ui + ti;
            s_re[jp] = ur - tr;
            s_im[jp] = ui - ti;
        }}
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }}
    output[2u * t] = s_re[t];
    output[2u * t + 1u] = s_im[t];
}}"#,
        wg = wg,
        entry = kernel.entry_point
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    Ok(())
}

fn msl_scalar(element: crate::wgsl_forge::ir::ScalarType) -> &'static str {
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
            Op::StructLoad {
                buffer,
                field,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = {buffer}.{field};")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Load {
                buffer,
                index,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = {buffer}[{index}];")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Store {
                buffer,
                index,
                value,
            } => {
                writeln!(source, "{indent}{buffer}[{index}] = {value};")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Fma {
                a,
                b,
                c,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = {a} * {b} + {c};")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Mul {
                left,
                right,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = {left} * {right};")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Add {
                left,
                right,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = {left} + {right};")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::DotProduct {
                left_buffer,
                left_base,
                right_buffer,
                right_base,
                len,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = 0.0;")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}for (uint i = 0; i < {len}; i++) {{")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}    {destination} += {left_buffer}[{left_base} + i] * {right_buffer}[{right_base} + i];").map_err(|error| ForgeError::Emission(error.to_string()))?;
                writeln!(source, "{indent}}}")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Loop {
                induction_var,
                start,
                end,
                step,
                body,
            } => {
                writeln!(source, "{indent}for (uint {induction_var} = {start}; {induction_var} < {end}; {induction_var} += {step}) {{").map_err(|error| ForgeError::Emission(error.to_string()))?;
                emit_ops(source, body, &format!("{indent}    "))?;
                writeln!(source, "{indent}}}")
                    .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Relu {
                operand,
                destination,
            } => {
                writeln!(
                    source,
                    "{indent}float {destination} = max(0.0f, {operand});"
                )
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Gelu {
                operand,
                destination,
            } => {
                writeln!(source, "{indent}float {destination} = 0.5f * {operand} * (1.0f + tanh(0.7978845608f * ({operand} + 0.044715f * {operand} * {operand} * {operand})));").map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::MatrixMultiply { .. } => {
                // No scalar MSL lowering for a dense GEMM op; fail loudly rather
                // than silently emit nothing (tensor-core GEMM is delivered elsewhere).
                return Err(ForgeError::Emission(
                    "Op::MatrixMultiply has no scalar MSL lowering; use the cooperative-matrix / CUDA WMMA path".to_string(),
                ));
            }
            Op::Barrier => {
                writeln!(
                    source,
                    "{indent}threadgroup_barrier(mem_flags::mem_threadgroup);"
                )
                .map_err(|error| ForgeError::Emission(error.to_string()))?;
            }
            Op::Intrinsic(_) => {
                return Err(ForgeError::Emission(
                    "Intrinsics not implemented for MSL yet".to_string(),
                ));
            }
        }
    }
    Ok(())
}
