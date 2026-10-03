//! Per-vertex surface colour reading.
//!
//! The quantized mesh section stores positions only. One organ albedo painted
//! in the shader is still one flat colour, even when a light shades the
//! faces. This section is the reading on the part: one RGBA8 sample per
//! vertex, in the same order as the mesh. It is not a texture, not a normal,
//! and not a game-specific dialect.

/// Magic `SRD1`.
pub const SURFACE_READING_MAGIC: &[u8; 4] = b"SRD1";

pub fn encoded_len(vertex_count: usize) -> usize {
    8 + vertex_count * 4
}

/// Quantize authored readings to RGBA8. Length must match the mesh.
pub fn encode_surface_reading(reading: &[[f32; 4]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encoded_len(reading.len()));
    out.extend_from_slice(SURFACE_READING_MAGIC);
    out.extend_from_slice(&(reading.len() as u32).to_le_bytes());
    for sample in reading {
        for channel in sample {
            let q = (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
            out.push(q);
        }
    }
    out
}

/// `None` when the payload is not a surface reading or the count is short.
pub fn decode_surface_reading(payload: &[u8]) -> Option<Vec<[f32; 4]>> {
    if payload.len() < 8 || &payload[..4] != SURFACE_READING_MAGIC {
        return None;
    }
    let count = u32::from_le_bytes(payload[4..8].try_into().ok()?) as usize;
    if payload.len() < 8 + count * 4 {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = 8 + i * 4;
        out.push([
            payload[o] as f32 / 255.0,
            payload[o + 1] as f32 / 255.0,
            payload[o + 2] as f32 / 255.0,
            payload[o + 3] as f32 / 255.0,
        ]);
    }
    Some(out)
}
