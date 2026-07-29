//! PNG decode → packed RGB8. Cold path (`Vec` allowed).

use super::super::error::CvError;

/// Decode a PNG byte stream into packed RGB8 plus dimensions.
///
/// Returns `(rgb, width, height)` where `rgb.len() == width * height * 3`.
/// Any PNG colour type is normalised to RGB8: RGB is copied, RGBA drops the
/// alpha channel, grayscale replicates the single sample across R/G/B, and
/// grayscale+alpha replicates luma while dropping alpha. Palette/low-bit-depth
/// and 16-bit inputs are expanded/stripped to 8-bit via `normalize_to_color8`.
///
/// Cold path only. Any decode failure maps to a [`CvError`] — never panics.
pub fn decode_png(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), CvError> {
    if bytes.is_empty() {
        return Err(CvError::EmptyInput);
    }

    let mut decoder = png::Decoder::new(bytes);
    // EXPAND (palette→RGB, sub-8-bit gray→8-bit, tRNS→alpha) | STRIP_16 (→8-bit).
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().map_err(|_| CvError::InvalidParameter)?;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|_| CvError::InvalidParameter)?;

    let width = info.width;
    let height = info.height;
    if width == 0 || height == 0 {
        return Err(CvError::EmptyInput);
    }

    let px = (width as usize)
        .checked_mul(height as usize)
        .ok_or(CvError::InvalidParameter)?;

    // Number of source samples per pixel for the normalised colour type.
    let src_channels = match info.color_type {
        png::ColorType::Rgb => 3usize,
        png::ColorType::Rgba => 4usize,
        png::ColorType::Grayscale => 1usize,
        png::ColorType::GrayscaleAlpha => 2usize,
        // After normalize_to_color8, Indexed should have been expanded away.
        png::ColorType::Indexed => return Err(CvError::InvalidParameter),
    };

    let need = px
        .checked_mul(src_channels)
        .ok_or(CvError::InvalidParameter)?;
    if buf.len() < need {
        return Err(CvError::BufferTooSmall);
    }

    let mut rgb = vec![0u8; px * 3];
    match info.color_type {
        png::ColorType::Rgb => {
            rgb[..px * 3].copy_from_slice(&buf[..px * 3]);
        }
        png::ColorType::Rgba => {
            for i in 0..px {
                let s = i * 4;
                let d = i * 3;
                rgb[d] = buf[s];
                rgb[d + 1] = buf[s + 1];
                rgb[d + 2] = buf[s + 2];
            }
        }
        png::ColorType::Grayscale => {
            for i in 0..px {
                let g = buf[i];
                let d = i * 3;
                rgb[d] = g;
                rgb[d + 1] = g;
                rgb[d + 2] = g;
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..px {
                let g = buf[i * 2];
                let d = i * 3;
                rgb[d] = g;
                rgb[d + 1] = g;
                rgb[d + 2] = g;
            }
        }
        // Expanded away by normalize_to_color8; already rejected above.
        png::ColorType::Indexed => return Err(CvError::InvalidParameter),
    }

    Ok((rgb, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_png(w: u32, h: u32, color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(color);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().expect("header");
            writer.write_image_data(data).expect("data");
            writer.finish().expect("finish");
        }
        out
    }

    #[test]
    fn grayscale_png_decodes_to_replicated_rgb() {
        let (w, h) = (2u32, 2u32);
        let gray = [10u8, 20, 30, 40]; // one sample per pixel
        let png = build_png(w, h, png::ColorType::Grayscale, &gray);
        let (rgb, dw, dh) = decode_png(&png).expect("decode ok");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(rgb, vec![10, 10, 10, 20, 20, 20, 30, 30, 30, 40, 40, 40]);
    }

    #[test]
    fn rgba_png_decodes_dropping_alpha() {
        let (w, h) = (2u32, 1u32);
        // Two RGBA pixels; alpha (99, 0) must be dropped.
        let rgba = [1u8, 2, 3, 99, 4, 5, 6, 0];
        let png = build_png(w, h, png::ColorType::Rgba, &rgba);
        let (rgb, dw, dh) = decode_png(&png).expect("decode ok");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(rgb, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn malformed_png_fails_closed() {
        let garbage = [0x89u8, b'P', b'N', b'G', 0xff, 0xff, 0xff, 0xff, 0x00, 0x01];
        let r = decode_png(&garbage);
        assert!(r.is_err(), "malformed PNG must return Err, got {:?}", r);
    }

    #[test]
    fn empty_png_is_empty_input() {
        assert_eq!(decode_png(&[]), Err(CvError::EmptyInput));
    }
}
