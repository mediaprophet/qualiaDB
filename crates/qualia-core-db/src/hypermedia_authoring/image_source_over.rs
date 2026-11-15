//! Straight RGBA8 source-over, in the existing encoded-sample working space.
//! This does not imply linear-light or ICC-managed compositing.
pub(super) fn rgba(source: [u8; 4], destination: [u8; 4], coverage: f32) -> [u8; 4] {
    let sa = source[3] as f32 / 255.0 * coverage.clamp(0.0, 1.0);
    let da = destination[3] as f32 / 255.0;
    let a = sa + da * (1.0 - sa);
    if a <= 0.0 {
        return [0; 4];
    }
    let mut out = [0; 4];
    for i in 0..3 {
        out[i] = ((source[i] as f32 * sa + destination[i] as f32 * da * (1.0 - sa)) / a)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    out[3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
    out
}
