//! CUDA-C source for the Q8_0 decode GEMV.

pub(crate) const Q8_0_GEMV_ENTRY: &str = "q8_0_gemv";
pub(crate) const Q8_0_GEMV_ROWS: usize = 8;

/// Eight warps produce eight output rows. The activation is only a few KiB for decode-shaped
/// models and remains cache-resident; direct warp loads avoid two block barriers per 32 columns.
pub(crate) const Q8_0_GEMV_SRC: &str = r#"
#include <cuda_fp16.h>
#define Q8_ROWS 8u

extern "C" __global__ void q8_0_gemv(const float *x,
                                      const unsigned char *w,
                                      float *y,
                                      const unsigned *dims) {
    const unsigned n_in = dims[0];
    const unsigned n_out = dims[1];
    const unsigned row_bytes = dims[2];
    const unsigned warp = threadIdx.x >> 5u;
    const unsigned lane = threadIdx.x & 31u;
    const unsigned row = blockIdx.x * Q8_ROWS + warp;
    float sum = 0.0f;

    for (unsigned col = 0u; col < n_in; col += 32u) {
        const float activation = x[col + lane];
        if (row < n_out) {
            const unsigned char *block = w + row * row_bytes + (col >> 5u) * 34u;
            const unsigned short scale_bits =
                (unsigned short)block[0] | ((unsigned short)block[1] << 8u);
            __half_raw scale_raw;
            scale_raw.x = scale_bits;
            const __half scale = scale_raw;
            const signed char q = (signed char)block[2u + lane];
            sum = fmaf(__half2float(scale) * (float)q, activation, sum);
        }
    }

    for (unsigned delta = 16u; delta > 0u; delta >>= 1u) {
        sum += __shfl_down_sync(0xffffffffu, sum, delta);
    }
    if (lane == 0u && row < n_out) {
        y[row] = sum;
    }
}
"#;
