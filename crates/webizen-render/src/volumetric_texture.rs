use qualia_core_db::render::gpu::TextureMipSemantic;

/// Reduce an RGBA8 source to one requested coarse level without allocating.
///
/// `output` must hold the exact requested level. `scratch_a` and `scratch_b` must each hold the
/// first downsampled level; later levels are smaller and reuse those buffers. The source is never
/// modified. This is the streaming primitive used when a decoder cannot provide an authored mip.
pub(crate) fn coarse_texture_level_into(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    first_mip: u8,
    semantic: TextureMipSemantic,
    output: &mut [u8],
    scratch_a: &mut [u8],
    scratch_b: &mut [u8],
) -> Result<(u32, u32), String> {
    use qualia_core_db::render::cpu_texture_mips::{
        downsample_alpha_mask_level_into, downsample_rgba8_level_into, CpuTextureMipSemantic,
    };

    fn rgba8_bytes(width: u32, height: u32) -> Result<usize, String> {
        u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or_else(|| "coarse texture byte size overflow".to_string())
    }

    fn reduce_level(
        authored_base: &[u8],
        base_width: u32,
        base_height: u32,
        source: &[u8],
        width: u32,
        height: u32,
        semantic: TextureMipSemantic,
        out: &mut [u8],
    ) -> Result<(u32, u32), String> {
        let reduced = match semantic {
            TextureMipSemantic::AlphaMask { cutoff_bits } => downsample_alpha_mask_level_into(
                authored_base,
                base_width,
                base_height,
                source,
                width,
                height,
                f32::from_bits(cutoff_bits),
                out,
            ),
            TextureMipSemantic::Color => downsample_rgba8_level_into(
                source,
                width,
                height,
                CpuTextureMipSemantic::Color,
                out,
            ),
            TextureMipSemantic::LinearData => downsample_rgba8_level_into(
                source,
                width,
                height,
                CpuTextureMipSemantic::LinearData,
                out,
            ),
            TextureMipSemantic::Normal => downsample_rgba8_level_into(
                source,
                width,
                height,
                CpuTextureMipSemantic::Normal,
                out,
            ),
        };
        reduced.map_err(|error| format!("coarse texture reduction: {error:?}"))
    }

    let source_bytes = rgba8_bytes(source_width, source_height)?;
    if source.len() != source_bytes {
        return Err("coarse texture source length mismatch".to_string());
    }
    let mut width = source_width;
    let mut height = source_height;
    for _ in 0..first_mip {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let output_bytes = rgba8_bytes(width, height)?;
    if output.len() < output_bytes {
        return Err("coarse texture output buffer is too small".to_string());
    }
    if first_mip == 0 {
        output[..output_bytes].copy_from_slice(source);
        return Ok((width, height));
    }

    let first_width = (source_width / 2).max(1);
    let first_height = (source_height / 2).max(1);
    let first_bytes = rgba8_bytes(first_width, first_height)?;
    if scratch_a.len() < first_bytes || scratch_b.len() < first_bytes {
        return Err("coarse texture scratch buffer is too small".to_string());
    }

    let mut previous_width = source_width;
    let mut previous_height = source_height;
    for level in 0..first_mip {
        let next_width = (previous_width / 2).max(1);
        let next_height = (previous_height / 2).max(1);
        let (previous, next) = if level == 0 {
            (source, &mut scratch_a[..first_bytes])
        } else if level % 2 == 1 {
            let previous_bytes = rgba8_bytes(previous_width, previous_height)?;
            (
                &scratch_a[..previous_bytes],
                &mut scratch_b[..first_bytes],
            )
        } else {
            let previous_bytes = rgba8_bytes(previous_width, previous_height)?;
            (
                &scratch_b[..previous_bytes],
                &mut scratch_a[..first_bytes],
            )
        };
        let result = reduce_level(
            source,
            source_width,
            source_height,
            previous,
            previous_width,
            previous_height,
            semantic,
            next,
        )?;
        if result != (next_width, next_height) {
            return Err("coarse texture reducer returned inconsistent dimensions".to_string());
        }
        previous_width = next_width;
        previous_height = next_height;
    }

    let final_level = if first_mip % 2 == 1 {
        &scratch_a[..output_bytes]
    } else {
        &scratch_b[..output_bytes]
    };
    output[..output_bytes].copy_from_slice(final_level);
    Ok((width, height))
}

pub(crate) fn coarse_texture_level(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    first_mip: u8,
    semantic: TextureMipSemantic,
) -> Result<(Vec<u8>, u32, u32), String> {
    let mut width = source_width;
    let mut height = source_height;
    for _ in 0..first_mip {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let coarse_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| "coarse texture size overflow".to_string())?;
    let mut coarse = Vec::new();
    coarse
        .try_reserve_exact(coarse_bytes)
        .map_err(|_| "coarse texture output allocation failed".to_string())?;
    coarse.resize(coarse_bytes, 0);
    if first_mip == 0 {
        coarse.copy_from_slice(source);
        return Ok((coarse, width, height));
    }
    let first_width = (source_width / 2).max(1);
    let first_height = (source_height / 2).max(1);
    let first_bytes = u64::from(first_width)
        .checked_mul(u64::from(first_height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| "coarse texture scratch size overflow".to_string())?;
    let mut level_a = Vec::new();
    let mut level_b = Vec::new();
    level_a
        .try_reserve_exact(first_bytes)
        .map_err(|_| "coarse texture scratch allocation failed".to_string())?;
    level_b
        .try_reserve_exact(first_bytes)
        .map_err(|_| "coarse texture scratch allocation failed".to_string())?;
    level_a.resize(first_bytes, 0);
    level_b.resize(first_bytes, 0);
    coarse_texture_level_into(
        source,
        source_width,
        source_height,
        first_mip,
        semantic,
        &mut coarse,
        &mut level_a,
        &mut level_b,
    )?;
    Ok((coarse, width, height))
}
