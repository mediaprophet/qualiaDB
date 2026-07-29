// [WASM] Intentionally empty. coop_gemv_subgroup.wgsl uses subgroupAdd and subgroup builtins.
// On WASM/WebGPU, subgroups are not supported. The native pipeline concatenates
// the original (shaders/coop_gemv_subgroup.wgsl) only when SUBGROUP is available.
