//! Bounded, caller-output image layer composition in encoded RGBA8 space.
//!
//! Blend equations follow W3C Compositing and Blending Level 1, section 10.
//! This is an artistic encoded-sample operator, not colour management.

use super::{source_over, BlendMode, ImageDocument, ImageError, ImageLimits};

/// Validate the complete source, including hidden layers, before writing output.
/// Returns the exact number of document pixels required in the caller buffer.
pub fn validate_document(doc: &ImageDocument, limits: ImageLimits) -> Result<usize, ImageError> {
    let pixels = checked_pixels(doc.width, doc.height, limits)?;
    let mut visits = pixels;
    let mut bytes = pixels.checked_mul(4).ok_or(ImageError::Budget)?;
    for layer in &doc.layers {
        let layer_pixels = checked_pixels(layer.width, layer.height, limits)?;
        if layer.pixels.len() != layer_pixels ||
            layer.mask.as_ref().is_some_and(|mask| mask.len() != layer_pixels)
        {
            return Err(ImageError::Storage);
        }
        if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
            return Err(ImageError::Parameters);
        }
        visits = visits.checked_add(layer_pixels).ok_or(ImageError::Budget)?;
        let layer_bytes = layer_pixels.checked_mul(4).ok_or(ImageError::Budget)?;
        bytes = bytes.checked_add(layer_bytes).ok_or(ImageError::Budget)?;
        if layer.mask.is_some() {
            bytes = bytes.checked_add(layer_pixels).ok_or(ImageError::Budget)?;
        }
        if visits > limits.max_work || bytes > limits.max_bytes {
            return Err(ImageError::Budget);
        }
    }
    if visits > limits.max_work || bytes > limits.max_bytes {
        return Err(ImageError::Budget);
    }
    Ok(pixels)
}

fn checked_pixels(width: u32, height: u32, limits: ImageLimits) -> Result<usize, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::Dimensions);
    }
    let count = (width as usize).checked_mul(height as usize).ok_or(ImageError::Dimensions)?;
    if count > limits.max_pixels {
        return Err(ImageError::Dimensions);
    }
    Ok(count)
}

/// Compose into caller-owned storage. Every refusal occurs before the first write.
pub fn composite_into(doc: &ImageDocument, out: &mut [[u8; 4]], limits: ImageLimits) -> Result<(), ImageError> {
    let count = validate_document(doc, limits)?;
    if out.len() != count {
        return Err(ImageError::Output);
    }
    out.fill([0; 4]);
    let dst_width = doc.width as usize;
    for layer in &doc.layers {
        if !layer.visible { continue; }
        let x0 = (layer.offset_x as i64).max(0);
        let y0 = (layer.offset_y as i64).max(0);
        let x1 = (layer.offset_x as i64 + layer.width as i64).min(doc.width as i64);
        let y1 = (layer.offset_y as i64 + layer.height as i64).min(doc.height as i64);
        if x1 <= x0 || y1 <= y0 { continue; }
        for y in y0..y1 {
            let local_y = (y - layer.offset_y as i64) as usize;
            for x in x0..x1 {
                let local_x = (x - layer.offset_x as i64) as usize;
                let source_index = local_y * layer.width as usize + local_x;
                let dest_index = y as usize * dst_width + x as usize;
                let coverage = layer.opacity * layer.mask.as_ref()
                    .map_or(1.0, |mask| mask[source_index] as f32 / 255.0);
                let source = layer.pixels[source_index];
                let backdrop = out[dest_index];
                out[dest_index] = if layer.blend_mode == BlendMode::Normal {
                    // This preserves the established Normal byte rounding, including
                    // unmasked zero-offset legacy documents.
                    source_over::rgba(source, backdrop, coverage)
                } else {
                    blend_over(source, backdrop, coverage, layer.blend_mode)
                };
            }
        }
    }
    Ok(())
}

fn blend_over(source: [u8; 4], backdrop: [u8; 4], coverage: f32, mode: BlendMode) -> [u8; 4] {
    let sa = source[3] as f32 / 255.0 * coverage;
    let ba = backdrop[3] as f32 / 255.0;
    let alpha = sa + ba * (1.0 - sa);
    if alpha <= 0.0 { return [0; 4]; }
    let mut result = [0; 4];
    for channel in 0..3 {
        let cs = source[channel] as f32 / 255.0;
        let cb = backdrop[channel] as f32 / 255.0;
        // The backdrop-alpha interpolation makes blend modes reduce to the
        // unmodified source over a transparent backdrop.
        let blended_source = (1.0 - ba) * cs + ba * blend_channel(cb, cs, mode);
        let premultiplied = sa * blended_source + ba * (1.0 - sa) * cb;
        result[channel] = ((premultiplied / alpha) * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    result[3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    result
}

fn blend_channel(cb: f32, cs: f32, mode: BlendMode) -> f32 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                let d = if cb <= 0.25 {
                    ((16.0 * cb - 12.0) * cb + 4.0) * cb
                } else {
                    cb.sqrt()
                };
                cb + (2.0 * cs - 1.0) * (d - cb)
            }
        }
        BlendMode::ColorDodge => {
            if cb <= 0.0 { 0.0 }
            else if cs >= 1.0 { 1.0 }
            else { (cb / (1.0 - cs)).min(1.0) }
        }
        BlendMode::ColorBurn => {
            if cb >= 1.0 { 1.0 }
            else if cs <= 0.0 { 0.0 }
            else { 1.0 - ((1.0 - cb) / cs).min(1.0) }
        }
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { 2.0 * cb * cs } else { 1.0 - 2.0 * (1.0 - cb) * (1.0 - cs) }
}
