use etalon_model::ggml::*;

pub const TYPES: [(u32, usize, usize); 13] = [
    (0, 1, 4),
    (1, 1, 2),
    (2, 32, 18),
    (6, 32, 22),
    (8, 32, 34),
    (12, 256, 144),
    (13, 256, 176),
    (14, 256, 210),
    (20, 32, 18),
    (23, 256, 136),
    (30, 1, 2),
    (40, 1, 1),
    (112, 256, 160),
];
pub const LENGTHS: [usize; 9] = [0, 1, 31, 32, 33, 255, 256, 257, 513];
pub const SENTINEL: f32 = f32::from_bits(0x7fc12345);

#[repr(align(2))]
pub struct Raw(pub [u8; 4096]);
impl Raw {
    pub fn new() -> Self {
        let mut result = Self([0; 4096]);
        for (i, b) in result.0.iter_mut().enumerate() {
            *b = ((i * 37 + i / 7 + 11) % 256) as u8;
        }
        result
    }
}
pub fn hash(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(1099511628211);
    }
    h
}
pub fn row_fingerprint(dtype: u32, block: usize, bytes: usize) -> u64 {
    let raw = Raw::new();
    let mut h = 14695981039346656037;
    for n in LENGTHS {
        let size = n.div_ceil(block) * bytes;
        assert_eq!(ggml_row_bytes(dtype, n), Some(size));
        for short in [false, true] {
            let len = if short { size.saturating_sub(1) } else { size };
            let mut out = [SENTINEL; 520];
            let result = dequantize_row_into(&raw.0[..len], dtype, n, &mut out);
            let code = match result {
                Ok(count) => count as u64,
                Err(GgmlDequantError::TruncatedInput) => u64::MAX,
                other => panic!("unexpected {dtype}/{n}: {other:?}"),
            };
            h = hash(h, &code.to_le_bytes());
            for value in out {
                h = hash(h, &value.to_bits().to_le_bytes());
            }
        }
    }
    h
}
pub fn conversion_fingerprint() -> u64 {
    let raw = Raw::new();
    let mut h = 14695981039346656037;
    let mut out = [0xa5; 1300];
    q4k_block_to_soa(&raw.0[..144], &mut out).unwrap();
    h = hash(h, &out);
    expand_q4k_tensor_to_soa(&raw.0[..864], 513, 2, &mut out).unwrap();
    h = hash(h, &out);
    let mut src = [0.0; 1026];
    for (i, v) in src.iter_mut().enumerate() {
        *v = ((i * 17 % 137) as f32 - 68.0) / 13.0;
    }
    for nonfinite in [false, true] {
        if nonfinite {
            src[0] = f32::NAN;
            src[34] = f32::INFINITY;
            src[512] = f32::NEG_INFINITY;
            src[513] = -0.0;
        }
        out.fill(0xa5);
        quantize_f32_to_q4_k_soa_tensor(&src, 513, 2, &mut out).unwrap();
        h = hash(h, &out);
    }
    h
}
