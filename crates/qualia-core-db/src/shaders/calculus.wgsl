// Calculus Compute Shader
// 
// Pre-compiled WGSL shader for numerical integration on GPU.
// Implements Simpson's rule for integrating continuous data grids.
//
// Entry Points:
//   - simpsons_integration: Simpson's rule integration
//   - trapezoidal_integration: Trapezoidal rule integration (fallback)
//
// Bindings:
//   - binding 0: Input data buffer (read-only storage)
//   - binding 1: Workgroup reduction buffer (storage, one f64 per workgroup)
//   - binding 2: Step size uniform (f32)
//
// Precision Note: WGSL does not support f64 atomics. We use tree-reduction
// within each workgroup to produce a single f64 per workgroup, then the host
// CPU sums these smaller values using Kahan summation for full precision.

struct Uniforms {
    step_size: f32,
    total_elements: u32,
};

@group(0) @binding(0)
var<storage, read> input_data: array<f64>;

@group(0) @binding(1)
var<storage, read_write> workgroup_results: array<f64>;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

// Workgroup-local shared memory for tree reduction
var<workgroup> local_sum: array<f64, 64>;

@compute @workgroup_size(64, 1, 1)
fn simpsons_integration(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>, @builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let global_idx = global_id.x;
    let local_idx = local_id.x;
    let total_n = uniforms.total_elements;
    
    // Initialize local sum
    local_sum[local_idx] = 0.0;
    
    if (global_idx < total_n) {
        let step = uniforms.step_size;
        let value = input_data[global_idx];
        
        // Simpson's rule weights
        var weight: f32 = 1.0;
        if (global_idx == 0u || global_idx == total_n - 1u) {
            weight = 1.0;
        } else if (global_idx % 2u == 1u) {
            weight = 4.0;
        } else {
            weight = 2.0;
        }
        
        local_sum[local_idx] = f64(weight) * value * f64(step) / 3.0;
    }
    
    // Synchronize all threads in workgroup
    workgroupBarrier();
    
    // Tree reduction: reduce 64 values to 1
    // Stride 32, 16, 8, 4, 2, 1
    var stride: u32 = 32u;
    loop {
        if (local_idx < stride) {
            local_sum[local_idx] = local_sum[local_idx] + local_sum[local_idx + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if (stride == 0u) {
            break;
        }
    }
    
    // Thread 0 writes the workgroup result to global buffer
    if (local_idx == 0u) {
        workgroup_results[workgroup_id.x] = local_sum[0];
    }
}

@compute @workgroup_size(64, 1, 1)
fn trapezoidal_integration(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>, @builtin(workgroup_id) workgroup_id: vec3<u32>) {
    let global_idx = global_id.x;
    let local_idx = local_id.x;
    let total_n = uniforms.total_elements;
    
    // Initialize local sum
    local_sum[local_idx] = 0.0;
    
    if (global_idx < total_n) {
        let step = uniforms.step_size;
        let value = input_data[global_idx];
        
        // Trapezoidal rule weights
        var weight: f32 = 1.0;
        if (global_idx == 0u || global_idx == total_n - 1u) {
            weight = 1.0;
        } else {
            weight = 2.0;
        }
        
        local_sum[local_idx] = f64(weight) * value * f64(step) / 2.0;
    }
    
    // Synchronize all threads in workgroup
    workgroupBarrier();
    
    // Tree reduction: reduce 64 values to 1
    var stride: u32 = 32u;
    loop {
        if (local_idx < stride) {
            local_sum[local_idx] = local_sum[local_idx] + local_sum[local_idx + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if (stride == 0u) {
            break;
        }
    }
    
    // Thread 0 writes the workgroup result to global buffer
    if (local_idx == 0u) {
        workgroup_results[workgroup_id.x] = local_sum[0];
    }
}

// Workgroup-local shared memory for RK4 tree reduction (separate from Simpson's local_sum)
var<workgroup> rk4_local: array<f64, 64>;

// RK4 ODE quadrature entry point.
//
// Applies the 3/8 Simpson composite rule (weights 3h/8·[1,3,3,1] per group of 4 samples),
// which matches the classical four-stage RK4 error order O(h⁴).  Each thread handles one
// sample; groups of 4 consecutive threads collaborate on a single step.
//
// Output: one f64 per workgroup written to workgroup_results[], summed on the CPU via Kahan.
@compute @workgroup_size(64, 1, 1)
fn rk4_step(
    @builtin(global_invocation_id) global_id:    vec3<u32>,
    @builtin(local_invocation_id)  local_id:     vec3<u32>,
    @builtin(workgroup_id)         workgroup_id: vec3<u32>,
) {
    let global_idx = global_id.x;
    let local_idx  = local_id.x;
    let total_n    = uniforms.total_elements;

    rk4_local[local_idx] = 0.0;

    if global_idx < total_n {
        let h     = uniforms.step_size;
        let value = input_data[global_idx];

        // 3/8-rule weights cycling over a group of 4: pos = global_idx % 4
        //   pos 0 → weight 1  (k1)
        //   pos 1 → weight 3  (k2)
        //   pos 2 → weight 3  (k3)
        //   pos 3 → weight 1  (k4)
        // Boundary-endpoint positions share the "1" weight without doubling because
        // adjacent groups independently sum their endpoints.
        let pos_in_group = global_idx % 4u;
        var weight: f64;
        if pos_in_group == 1u || pos_in_group == 2u {
            weight = 3.0;
        } else {
            weight = 1.0;
        }

        // 3h/8 scaling factor
        rk4_local[local_idx] = weight * value * f64(h) * 3.0 / 8.0;
    }

    workgroupBarrier();

    // Tree reduction: 64 → 1 value
    var stride: u32 = 32u;
    loop {
        if local_idx < stride {
            rk4_local[local_idx] = rk4_local[local_idx] + rk4_local[local_idx + stride];
        }
        workgroupBarrier();
        stride = stride / 2u;
        if stride == 0u { break; }
    }

    if local_idx == 0u {
        workgroup_results[workgroup_id.x] = rk4_local[0];
    }
}
