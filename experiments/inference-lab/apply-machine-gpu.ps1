# Machine GPU capability profile: prefer native over WGSL-only defaults
$env:QUALIA_WGPU_BACKEND='spirv-dxc-vulkan'
$env:QUALIA_INFERENCE_MODE='portable'
# Forge: CUDA densify decode GEMV stays lab-only unless you know the package
# $env:QUALIA_LLM_CUDA_TC_DECODE='1'  # only after oracle green for that layout
