use qualia_core_db::render::gpu::TextureMipSemantic;

pub(crate) fn coarse_texture_level(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    first_mip: u8,
    semantic: TextureMipSemantic,
) -> Result<(Vec<u8>, u32, u32), String> {
    use qualia_core_db::render::cpu_texture_mips::{
        downsample_alpha_mask_level_into, downsample_rgba8_level_into, CpuTextureMipSemantic,
    };

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
    let mut width = source_width;
    let mut height = source_height;
    for _ in 0..first_mip {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let coarse_bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "coarse texture size overflow".to_string())?;
    let first_width = (source_width / 2).max(1);
    let first_height = (source_height / 2).max(1);
    let first_bytes = (first_width as usize)
        .checked_mul(first_height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
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

    let mut previous_width = source_width;
    let mut previous_height = source_height;
    for level in 0..first_mip {
        let next_width = (previous_width / 2).max(1);
        let next_height = (previous_height / 2).max(1);
        let result = if level == 0 {
            reduce_level(
                source,
                source_width,
                source_height,
                source,
                previous_width,
                previous_height,
                semantic,
                &mut level_a,
            )
        } else if level % 2 == 1 {
            let previous_bytes = (previous_width as usize)
                .checked_mul(previous_height as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| "coarse texture level size overflow".to_string())?;
            reduce_level(
                source,
                source_width,
                source_height,
                &level_a[..previous_bytes],
                previous_width,
                previous_height,
                semantic,
                &mut level_b,
            )
        } else {
            let previous_bytes = (previous_width as usize)
                .checked_mul(previous_height as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| "coarse texture level size overflow".to_string())?;
            reduce_level(
                source,
                source_width,
                source_height,
                &level_b[..previous_bytes],
                previous_width,
                previous_height,
                semantic,
                &mut level_a,
            )
        }?;
        if result != (next_width, next_height) {
            return Err("coarse texture reducer returned inconsistent dimensions".to_string());
        }
        previous_width = next_width;
        previous_height = next_height;
    }
    let coarse = if first_mip % 2 == 1 {
        level_a.truncate(coarse_bytes);
        level_a
    } else {
        level_b.truncate(coarse_bytes);
        level_b
    };
    Ok((coarse, width, height))
}
