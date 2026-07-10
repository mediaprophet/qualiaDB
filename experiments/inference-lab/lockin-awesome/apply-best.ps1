# Apply best measured config for 3B SoA on A2000 (2026-07-10)
$env:QUALIA_INFERENCE_MODE='cuda'
$env:QUALIA_WGPU_BACKEND='vulkan'
$env:QUALIA_LLM_RESIDENT_DECODE='1'
$env:QUALIA_LLM_COOP_GEMV='1'
$env:QUALIA_LLM_FFN_FUSION='1'
$env:QUALIA_LLM_KV_INT8='1'
# Smol interactive: use fast-verify instead (~63 tok/s)
# $env:QUALIA_INFERENCE_MODE='fast-verify'
