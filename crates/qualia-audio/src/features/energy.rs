//! Time-domain features.

/// RMS energy of mono samples.
pub fn frame_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut s = 0.0f32;
    for &x in samples {
        s += x * x;
    }
    (s / samples.len() as f32).sqrt()
}

/// Zero-crossing rate (fraction of sign changes).
pub fn frame_zcr(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut z = 0u32;
    for i in 1..samples.len() {
        if (samples[i - 1] >= 0.0) != (samples[i] >= 0.0) {
            z += 1;
        }
    }
    z as f32 / (samples.len() - 1) as f32
}
