/// Convert an FP8 E4M3 byte to f32.
///
/// Format: 1 sign bit, 4 exponent bits (bias 7), 3 mantissa bits.
#[inline(always)]
pub fn fp8_e4m3_to_f32(byte: u8) -> f32 {
    let sign = if (byte & 0x80) != 0 { -1.0f32 } else { 1.0f32 };
    let exp = ((byte >> 3) & 0x0F) as i32;
    let mant = (byte & 0x07) as f32;

    if exp == 0 {
        // Subnormal: (-1)^s * 2^(-6) * (m / 8)
        sign * (1.0 / 64.0) * (mant / 8.0)
    } else if exp == 15 && (byte & 0x07) == 0x07 {
        // NaN / Inf in E4M3
        0.0
    } else {
        // Normal: (-1)^s * 2^(exp - 7) * (1 + m / 8)
        let scale = 2.0f32.powi(exp - 7);
        sign * scale * (1.0 + mant / 8.0)
    }
}
