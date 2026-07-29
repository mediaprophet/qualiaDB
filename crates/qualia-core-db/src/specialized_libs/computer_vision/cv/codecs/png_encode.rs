//! RGB8 view → PNG byte stream. Cold path (`Vec` allowed).

use super::super::buffer::RgbView;
use super::super::error::CvError;

/// Encode a packed RGB8 [`RgbView`] as a PNG, returning the encoded bytes.
///
/// Writes an 8-bit RGB PNG. The view's `stride` may exceed `width * 3`
/// (padded rows); only the `width * 3` leading bytes of each row are written,
/// so the output is always tightly packed. Lossless: `decode_png` on the
/// result recovers the exact RGB samples.
///
/// Cold path only. Any encode failure maps to a [`CvError`] — never panics.
pub fn encode_png(rgb: RgbView<'_>) -> Result<Vec<u8>, CvError> {
    let width = rgb.width;
    let height = rgb.height;
    if width == 0 || height == 0 {
        return Err(CvError::EmptyInput);
    }

    let row_bytes = (width as usize)
        .checked_mul(3)
        .ok_or(CvError::InvalidParameter)?;
    let packed_len = row_bytes
        .checked_mul(height as usize)
        .ok_or(CvError::InvalidParameter)?;

    // Tightly pack rows (drop any stride padding) for the encoder.
    let mut packed = vec![0u8; packed_len];
    let stride = rgb.stride as usize;
    for y in 0..height as usize {
        let src = y * stride;
        let dst = y * row_bytes;
        let end = src
            .checked_add(row_bytes)
            .ok_or(CvError::InvalidParameter)?;
        if end > rgb.bytes.len() {
            return Err(CvError::BufferTooSmall);
        }
        packed[dst..dst + row_bytes].copy_from_slice(&rgb.bytes[src..end]);
    }

    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| CvError::InvalidParameter)?;
        writer
            .write_image_data(&packed)
            .map_err(|_| CvError::InvalidParameter)?;
        writer.finish().map_err(|_| CvError::InvalidParameter)?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::png_decode::decode_png;
    use super::*;

    /// PNG is lossless: encode → decode must recover the exact RGB samples.
    #[test]
    fn png_round_trip_is_byte_exact() {
        let (w, h) = (4u32, 4u32);
        // Known gradient: r = 16*x, g = 16*y, b = x^y scaled.
        let mut src = vec![0u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                src[i] = (x * 16) as u8;
                src[i + 1] = (y * 16) as u8;
                src[i + 2] = ((x ^ y) * 32) as u8;
            }
        }
        let view = RgbView::new(w, h, w * 3, &src).expect("valid view");
        let png = encode_png(view).expect("encode ok");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);

        let (rgb, dw, dh) = decode_png(&png).expect("decode ok");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(rgb, src, "round-trip must be byte-exact");
    }

    /// A padded stride (row bytes > width*3) is packed correctly on encode.
    #[test]
    fn png_round_trip_with_stride_padding() {
        let (w, h) = (3u32, 2u32);
        let stride = w * 3 + 5; // padded rows
        let mut src = vec![0u8; (stride * h) as usize];
        let mut expected = vec![0u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let base = (y * stride + x * 3) as usize;
                let (r, g, b) = ((x * 40) as u8, (y * 80) as u8, 200u8);
                src[base] = r;
                src[base + 1] = g;
                src[base + 2] = b;
                let e = ((y * w + x) * 3) as usize;
                expected[e] = r;
                expected[e + 1] = g;
                expected[e + 2] = b;
            }
        }
        let view = RgbView::new(w, h, stride, &src).expect("valid view");
        let png = encode_png(view).expect("encode ok");
        let (rgb, dw, dh) = decode_png(&png).expect("decode ok");
        assert_eq!((dw, dh), (w, h));
        assert_eq!(rgb, expected);
    }

    #[test]
    fn png_encode_rejects_empty() {
        let bytes = [0u8; 3];
        let view = RgbView {
            width: 0,
            height: 1,
            stride: 3,
            bytes: &bytes,
        };
        assert_eq!(encode_png(view), Err(CvError::EmptyInput));
    }
}
