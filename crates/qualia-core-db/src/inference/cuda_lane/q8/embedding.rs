//! Device-resident Q8_0 token embedding lookup.
//!
//! The selected GGUF row is decoded directly into the mega-pass residual buffer. This removes
//! the CPU row dequantization and 3.75 KiB activation upload from each SmolLM2 decode step.

pub(crate) const Q8_0_EMBEDDING_LOOKUP_ENTRY: &str = "q8_0_embedding_lookup";

pub(crate) const Q8_0_EMBEDDING_LOOKUP_SRC: &str = r#"
#include <cuda_fp16.h>

extern "C" __global__ void q8_0_embedding_lookup(const unsigned char *weights,
                                                  float *hidden,
                                                  const unsigned *qkv_dims,
                                                  const unsigned *step) {
    const unsigned column = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned n_embd = qkv_dims[0];
    const unsigned row_bytes = qkv_dims[3];
    if (column >= n_embd) {
        return;
    }

    const unsigned token_id = step[2];
    const unsigned char *block =
        weights + token_id * row_bytes + (column >> 5u) * 34u;
    const unsigned short scale_bits =
        (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
    __half_raw scale_raw;
    scale_raw.x = scale_bits;
    const signed char quantized = (signed char)block[2u + (column & 31u)];
    hidden[column] = __half2float(scale_raw) * (float)quantized;
}
"#;
