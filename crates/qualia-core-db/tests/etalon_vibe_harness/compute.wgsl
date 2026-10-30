@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn add(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x < arrayLength(&output) {
        output[gid.x] = a[gid.x] + b[gid.x];
    }
}

@compute @workgroup_size(64)
fn sparse(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x < arrayLength(&output) && gid.x % 2u == 0u {
        output[gid.x] = a[gid.x] + b[gid.x];
    }
}
