//! Swarm S-V10 — image → heightfield mesh (reference reconstruction).
//!
//! Honest: monocular height from luminance gradient — not multi-view SfM or a
//! foundation image-to-3D network. Output is an **epistemic proposed mesh**;
//! must pass [`validate_mesh_ir`] before any Q42 commit.

use crate::semantic::{media_digest, q_hash, MediaDigest};
use crate::types::{ImageView, PixelFormat, VisionError};
use qualia_core_db::specialized_libs::computer_vision::spatial::{
    validate_mesh_ir, MeshIR, MeshValidationReport, MAX_VERTICES,
};

pub const RECON_MODEL_ID: &str = "qualia-vision-heightfield-recon-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageTo3dReceipt {
    pub model_hash: u64,
    pub source_digest: MediaDigest,
    pub mesh_hash: u64,
    pub grid_w: u32,
    pub grid_h: u32,
    /// Always true for this path until a licensed recon model is loaded.
    pub is_reference_recon: bool,
}

fn sample_luma(img: ImageView<'_>, x: u32, y: u32) -> f32 {
    let bpp = img.bytes_per_pixel() as usize;
    let off = (y as usize)
        .saturating_mul(img.row_stride as usize)
        .saturating_add((x as usize).saturating_mul(bpp));
    if off + bpp > img.bytes.len() {
        return 0.0;
    }
    match img.format {
        PixelFormat::Gray8 => img.bytes[off] as f32 / 255.0,
        PixelFormat::Rgb8 | PixelFormat::Rgba8 => {
            let r = img.bytes[off] as f32;
            let g = img.bytes[off + 1] as f32;
            let b = img.bytes[off + 2] as f32;
            (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
        }
        PixelFormat::Bgr8 => {
            let b = img.bytes[off] as f32;
            let g = img.bytes[off + 1] as f32;
            let r = img.bytes[off + 2] as f32;
            (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
        }
        PixelFormat::RgbF32 => 0.0,
    }
}

/// Build a heightfield mesh from image luminance. `grid` is samples per side (2..=64).
pub fn image_to_heightfield_mesh(
    image: ImageView<'_>,
    grid: u32,
) -> Result<(MeshIR, ImageTo3dReceipt, MeshValidationReport), VisionError> {
    if !image.is_well_formed() {
        return Err(VisionError::MalformedImage);
    }
    let g = grid.clamp(2, 64);
    let vc = (g * g) as usize;
    if vc > MAX_VERTICES {
        return Err(VisionError::OutputBufferTooSmall);
    }

    let mut mesh = MeshIR::empty();
    mesh.positions.reserve(vc);
    mesh.normals.reserve(vc);
    mesh.uvs.reserve(vc);
    mesh.indices
        .reserve((g as usize - 1) * (g as usize - 1) * 6);

    for j in 0..g {
        for i in 0..g {
            let u = i as f32 / (g - 1) as f32;
            let v = j as f32 / (g - 1) as f32;
            let px = (u * (image.width.saturating_sub(1)) as f32) as u32;
            let py = (v * (image.height.saturating_sub(1)) as f32) as u32;
            let z = sample_luma(image, px, py) * 0.35;
            let x = u * 2.0 - 1.0;
            let y = 1.0 - v * 2.0;
            mesh.positions.push([x, y, z]);
            mesh.normals.push([0.0, 0.0, 1.0]);
            mesh.uvs.push([u, v]);
        }
    }

    for j in 0..g - 1 {
        for i in 0..g - 1 {
            let i0 = j * g + i;
            let i1 = i0 + 1;
            let i2 = i0 + g;
            let i3 = i2 + 1;
            mesh.indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    // Recompute flat normals per vertex (average of faces — simple).
    for n in &mut mesh.normals {
        *n = [0.0, 0.0, 0.0];
    }
    let mut t = 0usize;
    while t + 2 < mesh.indices.len() {
        let a = mesh.indices[t] as usize;
        let b = mesh.indices[t + 1] as usize;
        let c = mesh.indices[t + 2] as usize;
        let pa = mesh.positions[a];
        let pb = mesh.positions[b];
        let pc = mesh.positions[c];
        let ab = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let ac = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        let nx = ab[1] * ac[2] - ab[2] * ac[1];
        let ny = ab[2] * ac[0] - ab[0] * ac[2];
        let nz = ab[0] * ac[1] - ab[1] * ac[0];
        for idx in [a, b, c] {
            mesh.normals[idx][0] += nx;
            mesh.normals[idx][1] += ny;
            mesh.normals[idx][2] += nz;
        }
        t += 3;
    }
    for n in &mut mesh.normals {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-8);
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
    }

    mesh.recompute_bounds_and_hash();
    let report = validate_mesh_ir(&mesh);
    let src = media_digest(image.bytes);
    let receipt = ImageTo3dReceipt {
        model_hash: q_hash(RECON_MODEL_ID),
        source_digest: src,
        mesh_hash: mesh.content_hash,
        grid_w: g,
        grid_h: g,
        is_reference_recon: true,
    };
    Ok((mesh, receipt, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PixelFormat;

    #[test]
    fn heightfield_validates() {
        let mut rgb = vec![0u8; 16 * 16 * 3];
        for y in 0..16 {
            for x in 0..16 {
                let i = (y * 16 + x) * 3;
                rgb[i] = (x * 16) as u8;
                rgb[i + 1] = (y * 16) as u8;
                rgb[i + 2] = 128;
            }
        }
        let img = ImageView {
            bytes: &rgb,
            width: 16,
            height: 16,
            row_stride: 48,
            format: PixelFormat::Rgb8,
        };
        let (mesh, rec, rep) = image_to_heightfield_mesh(img, 8).unwrap();
        assert!(rep.ok(), "{rep:?}");
        assert!(mesh.triangle_count() > 0);
        assert!(rec.is_reference_recon);
    }
}
