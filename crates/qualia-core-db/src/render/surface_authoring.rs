//! Deterministic tileable surface authoring. Callers own all public output.
//! This cold builder creates colour, tangent normal and packed ORM maps;
//! mip generation, transfer functions and texture residency are separate.

#[derive(Debug, Clone, Copy)]
pub struct SurfaceRecipe {
    pub seed: u32,
    /// Multiplicative sRGB tint; normally white when vertex colours supply paint.
    pub tint: [f32; 3],
    pub contrast: f32,
    pub relief: f32,
    /// Integer periods make the banding seamless across repeated UVs.
    pub bands: [u32; 2],
    pub band_strength: f32,
    pub roughness: f32,
    pub metallic: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceAuthoringError { InvalidRecipe, InvalidSize, OutputTooSmall, InvalidImage, Encode }

fn lattice(x: u32, y: u32, period: u32, seed: u32) -> f32 {
    let mut v = (x % period).wrapping_mul(0x9e37_79b9)
        ^ (y % period).wrapping_mul(0x85eb_ca6b) ^ seed;
    v ^= v >> 16; v = v.wrapping_mul(0x7feb_352d); v ^= v >> 15;
    (v & 0xffff) as f32 / 65535.0
}

fn noise(u: f32, v: f32, period: u32, seed: u32) -> f32 {
    let x = u.rem_euclid(1.0) * period as f32;
    let y = v.rem_euclid(1.0) * period as f32;
    let ix = x.floor() as u32; let iy = y.floor() as u32;
    let sx = x - x.floor(); let sy = y - y.floor();
    let sx = sx * sx * (3.0 - 2.0 * sx); let sy = sy * sy * (3.0 - 2.0 * sy);
    let a = lattice(ix, iy, period, seed);
    let b = lattice(ix + 1, iy, period, seed);
    let c = lattice(ix, iy + 1, period, seed);
    let d = lattice(ix + 1, iy + 1, period, seed);
    (a + (b - a) * sx) * (1.0 - sy) + (c + (d - c) * sx) * sy
}

fn height(r: SurfaceRecipe, u: f32, v: f32) -> f32 {
    let coarse = noise(u, v, 8, r.seed);
    let fine = noise(u, v, 32, r.seed ^ 0x51ed_270b);
    let wave = (std::f32::consts::TAU * (u * r.bands[0] as f32 + v * r.bands[1] as f32)
        + (coarse - 0.5) * 0.7).sin();
    (coarse * 0.55 + fine * 0.45 - 0.5) * (1.0 - r.band_strength)
        + wave * 0.5 * r.band_strength
}

fn byte(v: f32) -> u8 { (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 }

/// At most 12 MiB of map output (three 1024² RGBA8 images). Validate every
/// recipe and buffer before writing, so admission failures leave output intact.
pub fn generate_surface_maps_into(r: SurfaceRecipe, side: u32,
    colour: &mut [u8], normal: &mut [u8], orm: &mut [u8],
) -> Result<usize, SurfaceAuthoringError> {
    if !(8..=1024).contains(&side) { return Err(SurfaceAuthoringError::InvalidSize); }
    if r.tint.iter().chain([r.contrast, r.relief, r.band_strength, r.roughness, r.metallic].iter())
        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || r.bands.iter().any(|v| *v > 128)
    { return Err(SurfaceAuthoringError::InvalidRecipe); }
    let bytes = side as usize * side as usize * 4;
    if colour.len() < bytes || normal.len() < bytes || orm.len() < bytes {
        return Err(SurfaceAuthoringError::OutputTooSmall);
    }
    let step = 1.0 / side as f32;
    for y in 0..side { for x in 0..side {
        let u = x as f32 * step; let v = y as f32 * step;
        let h = height(r, u, v);
        let dx = (height(r, u + step, v) - height(r, u - step, v)) * r.relief / (2.0 * step);
        let dy = (height(r, u, v + step) - height(r, u, v - step)) * r.relief / (2.0 * step);
        let inv = (dx * dx + dy * dy + 1.0).sqrt().recip();
        let i = ((y * side + x) * 4) as usize;
        for c in 0..3 { colour[i + c] = byte(r.tint[c] * (1.0 - r.contrast * 0.5 + h * r.contrast)); }
        colour[i + 3] = 255;
        normal[i..i + 4].copy_from_slice(&[byte(0.5 - dx * inv * 0.5),
            byte(0.5 - dy * inv * 0.5), byte(0.5 + inv * 0.5), 255]);
        orm[i..i + 4].copy_from_slice(&[255, byte((r.roughness + h * 0.06).clamp(0.0, 1.0)),
            byte(r.metallic), 255]);
    }}
    Ok(bytes)
}

/// Ordinary UV01 source coordinates in metres before scene normalisation.
pub fn planar_uvs_into(positions: &[[f32; 3]], axes: [usize; 2], metres_per_repeat: f32,
    out: &mut [[f32; 2]],
) -> Result<usize, SurfaceAuthoringError> {
    if axes[0] > 2 || axes[1] > 2 || axes[0] == axes[1] || !metres_per_repeat.is_finite()
        || metres_per_repeat <= 0.0 || positions.iter().flatten().any(|v| !v.is_finite())
    { return Err(SurfaceAuthoringError::InvalidRecipe); }
    if out.len() < positions.len() { return Err(SurfaceAuthoringError::OutputTooSmall); }
    for (p, uv) in positions.iter().zip(out.iter_mut()) {
        *uv = [p[axes[0]] / metres_per_repeat, p[axes[1]] / metres_per_repeat];
    }
    Ok(positions.len())
}

/// Encode original RGBA8 artwork as standard PNG into a caller-owned buffer.
pub fn encode_surface_png_into(side: u32, rgba: &[u8], out: &mut [u8])
    -> Result<usize, SurfaceAuthoringError> {
    if !(8..=1024).contains(&side) || rgba.len() != side as usize * side as usize * 4 {
        return Err(SurfaceAuthoringError::InvalidImage);
    }
    // Worst-case deflate plus row filters/container chunks; fail before writing.
    if out.len() < rgba.len() + side as usize + 4096 { return Err(SurfaceAuthoringError::OutputTooSmall); }
    let mut cursor = std::io::Cursor::new(out);
    {
        let mut encoder = png::Encoder::new(&mut cursor, side, side);
        encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|_| SurfaceAuthoringError::Encode)?;
        writer.write_image_data(rgba).map_err(|_| SurfaceAuthoringError::Encode)?;
        writer.finish().map_err(|_| SurfaceAuthoringError::Encode)?;
    }
    Ok(cursor.position() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn recipe() -> SurfaceRecipe {
        SurfaceRecipe { seed: 7, tint: [1.0;3], contrast: 0.2, relief: 0.002,
            bands: [12,1], band_strength: 0.7, roughness: 0.8, metallic: 0.0 }
    }
    #[test]
    fn repeating_surface_is_continuous_and_buffer_rejection_is_atomic() {
        for v in [0.0,0.2,0.63,1.0] {
            assert!((height(recipe(),0.0,v)-height(recipe(),1.0,v)).abs()<1e-5);
        }
        let mut a=[9;256]; let mut b=[8;256]; let mut c=[7;255];
        assert_eq!(generate_surface_maps_into(recipe(),8,&mut a,&mut b,&mut c),
            Err(SurfaceAuthoringError::OutputTooSmall));
        assert_eq!(a,[9;256]); assert_eq!(b,[8;256]); assert_eq!(c,[7;255]);
    }
    #[test]
    fn png_round_trip_keeps_generated_normal_pixels() {
        let mut a=[0;256]; let mut b=[0;256]; let mut c=[0;256];
        generate_surface_maps_into(recipe(),8,&mut a,&mut b,&mut c).unwrap();
        let mut encoded=[0;8192]; let len=encode_surface_png_into(8,&b,&mut encoded).unwrap();
        let source=crate::render::texture_ingestion::ingest_texture_payload("image/png",Some(&encoded[..len]),
            crate::render::texture_decode::TextureDecodeLimits::default()).unwrap();
        let resource=crate::render::asset_package::HmcTextureResource { digest:source.digest,
            mime_type:&source.mime_type,bytes:&source.bytes };
        let mut decoded=[0;256]; let mut scratch=[0;256];
        crate::render::texture_decode::decode_hmc_texture_rgba8_into(&resource,
            crate::render::texture_decode::TextureDecodeLimits::default(),&mut decoded,&mut scratch).unwrap();
        assert_eq!(decoded,b);
    }
}
