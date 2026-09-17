//! Computer vision invoke seams — core CV operations.
//!
//! Exposes `specialized_libs::computer_vision` functions through VibeScript
//! invoke IDs in the `ComputerVision.*` namespace.

use super::super::args;
use vibe::{Diagnostic, Span, Value};

/// `ComputerVision.gaussian_blur` — fixed 3×3 Gaussian blur on a grayscale image.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64 }
pub fn gaussian_blur(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.gaussian_blur needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.gaussian_blur needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.gaussian_blur needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| {
                args::bad(
                    span,
                    "ComputerVision.gaussian_blur: invalid image dimensions",
                )
            })?;
    let mut out = vec![0u8; (width * height) as usize];
    match crate::specialized_libs::computer_vision::gaussian_blur_u8(view, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "output",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("gaussian_blur: {e:?}"))),
    }
}

/// `ComputerVision.sobel_magnitude` — Sobel edge-gradient magnitude on a grayscale image.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64 }
pub fn sobel_magnitude(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.sobel_magnitude needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.sobel_magnitude needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.sobel_magnitude needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| {
                args::bad(
                    span,
                    "ComputerVision.sobel_magnitude: invalid image dimensions",
                )
            })?;
    let mut out = vec![0u8; (width * height) as usize];
    match crate::specialized_libs::computer_vision::sobel_mag_u8(view, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "output",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("sobel_magnitude: {e:?}"))),
    }
}

/// `ComputerVision.canny_edges` — Canny edge detection on a grayscale image.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64, low_threshold: f64, high_threshold: f64 }
pub fn canny_edges(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.canny_edges needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.canny_edges needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.canny_edges needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let low = args::rec_u64(args, "low_threshold")
        .ok_or_else(|| args::bad(span, "ComputerVision.canny_edges needs low_threshold"))?
        as u8;
    let high = args::rec_u64(args, "high_threshold")
        .ok_or_else(|| args::bad(span, "ComputerVision.canny_edges needs high_threshold"))?
        as u8;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| {
                args::bad(span, "ComputerVision.canny_edges: invalid image dimensions")
            })?;
    let mut out = vec![0u8; (width * height) as usize];
    match crate::specialized_libs::computer_vision::canny_u8(view, low, high, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "edges",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("canny_edges: {e:?}"))),
    }
}

/// `ComputerVision.histogram` — compute a histogram of a grayscale image.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64, bins: u64 }
pub fn histogram(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.histogram needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.histogram needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.histogram needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| args::bad(span, "ComputerVision.histogram: invalid image dimensions"))?;
    let mut out = [0u32; 256];
    match crate::specialized_libs::computer_vision::histogram_u8(view, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "histogram",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("bins", Value::U64(256)),
        ])),
        Err(e) => Err(args::bad(span, format!("histogram: {e:?}"))),
    }
}

/// `ComputerVision.equalize_hist` — histogram equalization on a grayscale image.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64 }
pub fn equalize_hist(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.equalize_hist needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.equalize_hist needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.equalize_hist needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| {
                args::bad(
                    span,
                    "ComputerVision.equalize_hist: invalid image dimensions",
                )
            })?;
    let mut out = vec![0u8; (width * height) as usize];
    match crate::specialized_libs::computer_vision::equalize_hist_u8(view, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "output",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("equalize_hist: {e:?}"))),
    }
}

/// `ComputerVision.rgb_to_gray` — convert an RGB image to grayscale.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64 }
pub fn rgb_to_gray(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.rgb_to_gray needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.rgb_to_gray needs width"))?
        as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.rgb_to_gray needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64 * 3) as u32;
    let view = crate::specialized_libs::computer_vision::RgbView::new(width, height, stride, &data)
        .ok_or_else(|| args::bad(span, "ComputerVision.rgb_to_gray: invalid image dimensions"))?;
    let mut out = vec![0u8; (width * height) as usize];
    match crate::specialized_libs::computer_vision::rgb_to_gray_u8(view, &mut out) {
        Ok(()) => Ok(args::record([
            (
                "output",
                Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("width", Value::U64(width as u64)),
            ("height", Value::U64(height as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("rgb_to_gray: {e:?}"))),
    }
}

/// `ComputerVision.dhash` — difference hash for perceptual image hashing.
/// Args: { data: [u8], width: u64, height: u64, stride?: u64 }
pub fn dhash(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_u8_list(args, "data")
        .ok_or_else(|| args::bad(span, "ComputerVision.dhash needs data (u8 list)"))?;
    let width = args::rec_u64(args, "width")
        .ok_or_else(|| args::bad(span, "ComputerVision.dhash needs width"))? as u32;
    let height = args::rec_u64(args, "height")
        .ok_or_else(|| args::bad(span, "ComputerVision.dhash needs height"))?
        as u32;
    let stride = args::rec_u64(args, "stride").unwrap_or(width as u64) as u32;
    let view =
        crate::specialized_libs::computer_vision::GrayView::new(width, height, stride, &data)
            .ok_or_else(|| args::bad(span, "ComputerVision.dhash: invalid image dimensions"))?;
    match crate::specialized_libs::computer_vision::dhash_u64(view) {
        Ok(hash) => Ok(args::record([("dhash", Value::U64(hash))])),
        Err(e) => Err(args::bad(span, format!("dhash: {e:?}"))),
    }
}

/// `ComputerVision.hamming_distance` — Hamming distance between two u64 hashes.
/// Args: { a: u64, b: u64 }
pub fn hamming_distance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_u64(args, "a")
        .ok_or_else(|| args::bad(span, "ComputerVision.hamming_distance needs a"))?;
    let b = args::rec_u64(args, "b")
        .ok_or_else(|| args::bad(span, "ComputerVision.hamming_distance needs b"))?;
    let dist = crate::specialized_libs::computer_vision::hamming_distance_u64(a, b);
    Ok(args::record([(
        "hamming_distance",
        Value::U64(dist as u64),
    )]))
}

/// `ComputerVision.cosine_similarity` — cosine similarity between two embedding vectors.
/// Args: { a: [f64], b: [f64] }
pub fn cosine_similarity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "a")
        .ok_or_else(|| args::bad(span, "ComputerVision.cosine_similarity needs a"))?;
    let b = args::rec_f64_list(args, "b")
        .ok_or_else(|| args::bad(span, "ComputerVision.cosine_similarity needs b"))?;
    if a.len() != b.len() {
        return Err(args::bad(
            span,
            "ComputerVision.cosine_similarity: vectors must have equal length",
        ));
    }
    let a_f32: Vec<f32> = a.iter().map(|v| *v as f32).collect();
    let b_f32: Vec<f32> = b.iter().map(|v| *v as f32).collect();
    match crate::specialized_libs::computer_vision::cosine_similarity(&a_f32, &b_f32) {
        Ok(sim) => Ok(args::record([(
            "cosine_similarity",
            Value::F64(sim as f64),
        )])),
        Err(e) => Err(args::bad(span, format!("cosine_similarity: {e:?}"))),
    }
}
