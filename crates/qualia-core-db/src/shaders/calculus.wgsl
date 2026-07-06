// Portable f32 calculus reductions.
//
// Each entry writes one partial sum per 64-thread workgroup. The host performs
// the final reduction in f64 with Kahan compensation. WGSL/WebGPU does not
// expose portable f64 storage or arithmetic.

struct Uniforms {
    step_size: f32,
    total_elements: u32,
};

@group(0) @binding(0)
var<storage, read> input_data: array<f32>;

@group(0) @binding(1)
var<storage, read_write> workgroup_results: array<f32>;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

var<workgroup> local_sum: array<f32, 64>;

@compute @workgroup_size(64, 1, 1)
fn simpsons_integration(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let index = global_id.x;
    let lane = local_id.x;
    local_sum[lane] = 0.0;

    if index < uniforms.total_elements {
        var weight = 2.0;
        if index == 0u || index + 1u == uniforms.total_elements {
            weight = 1.0;
        } else if index % 2u == 1u {
            weight = 4.0;
        }
        local_sum[lane] = weight * input_data[index] * uniforms.step_size / 3.0;
    }

    workgroupBarrier();
    var stride = 32u;
    loop {
        if lane < stride {
            local_sum[lane] = local_sum[lane] + local_sum[lane + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if stride == 0u {
            break;
        }
    }
    if lane == 0u {
        workgroup_results[workgroup_id.x] = local_sum[0];
    }
}

@compute @workgroup_size(64, 1, 1)
fn trapezoidal_integration(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let index = global_id.x;
    let lane = local_id.x;
    local_sum[lane] = 0.0;

    if index < uniforms.total_elements {
        var weight = 2.0;
        if index == 0u || index + 1u == uniforms.total_elements {
            weight = 1.0;
        }
        local_sum[lane] = weight * input_data[index] * uniforms.step_size / 2.0;
    }

    workgroupBarrier();
    var stride = 32u;
    loop {
        if lane < stride {
            local_sum[lane] = local_sum[lane] + local_sum[lane + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if stride == 0u {
            break;
        }
    }
    if lane == 0u {
        workgroup_results[workgroup_id.x] = local_sum[0];
    }
}

// Composite Simpson 3/8 quadrature. This is intentionally not named RK4:
// quadrature samples are not Runge-Kutta derivative stages.
@compute @workgroup_size(64, 1, 1)
fn simpson_38_integration(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let index = global_id.x;
    let lane = local_id.x;
    local_sum[lane] = 0.0;

    if index < uniforms.total_elements {
        var weight = 3.0;
        if index == 0u || index + 1u == uniforms.total_elements {
            weight = 1.0;
        } else if index % 3u == 0u {
            weight = 2.0;
        }
        local_sum[lane] = 3.0 * uniforms.step_size * weight * input_data[index] / 8.0;
    }

    workgroupBarrier();
    var stride = 32u;
    loop {
        if lane < stride {
            local_sum[lane] = local_sum[lane] + local_sum[lane + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if stride == 0u {
            break;
        }
    }
    if lane == 0u {
        workgroup_results[workgroup_id.x] = local_sum[0];
    }
}
