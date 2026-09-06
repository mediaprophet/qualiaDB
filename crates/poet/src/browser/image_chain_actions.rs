//! Dual-path Tool Chest actions for curated `ComputerVision.*` ALL_BOUND ids.
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::json;
use web_sys::{Document, Element};

const MAX_LIVE_PIXELS: u64 = 512 * 512;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn need_container(document: &Document, label: &str, message: &str) -> Option<Element> {
    match selected_container(document) {
        Some(container) => Some(container),
        None => {
            super::interactions::show_tool_status(document, label, message, "error");
            None
        }
    }
}

fn parse_u8_csv(raw: &str) -> Vec<u8> {
    raw.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';'))
        .filter_map(|token| token.trim().parse::<u8>().ok())
        .take((MAX_LIVE_PIXELS as usize) * 3)
        .collect()
}

fn parse_f64_csv(raw: &str) -> Vec<f64> {
    raw.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

fn parse_u64_attr(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }
    trimmed.parse::<u64>().ok()
}

fn grayscale_dims(container: &Element) -> Option<(Vec<u8>, u64, u64)> {
    let width = container
        .get_attribute("data-pixel-width")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|w| *w > 0)?;
    let height = container
        .get_attribute("data-pixel-height")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|h| *h > 0)?;
    let expected = width.checked_mul(height)?;
    if expected > MAX_LIVE_PIXELS {
        return None;
    }
    let raw = container.get_attribute("data-grayscale-u8")?;
    let data = parse_u8_csv(&raw);
    if data.len() as u64 != expected {
        return None;
    }
    Some((data, width, height))
}

fn rgb_dims(container: &Element) -> Option<(Vec<u8>, u64, u64)> {
    let width = container
        .get_attribute("data-pixel-width")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|w| *w > 0)?;
    let height = container
        .get_attribute("data-pixel-height")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|h| *h > 0)?;
    let expected = width.checked_mul(height)?.checked_mul(3)?;
    if width.saturating_mul(height) > MAX_LIVE_PIXELS {
        return None;
    }
    let raw = container.get_attribute("data-rgb-u8")?;
    let data = parse_u8_csv(&raw);
    if data.len() as u64 != expected {
        return None;
    }
    Some((data, width, height))
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: Option<serde_json::Value>,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    let Some(args) = args else {
        super::interactions::show_tool_status(
            document,
            &label,
            &local_message,
            "unavailable",
        );
        return;
    };
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

/// `ComputerVision.histogram` — greyscale tone distribution.
pub(super) fn run_image_histogram(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a picture surface before running a histogram.",
    ) else {
        return;
    };
    let pixels = grayscale_dims(&container);
    let (sketch, args) = match pixels {
        Some((data, width, height)) => (
            "Local histogram sketch: greyscale pixels are present on this surface. Connect QualiaDB for ComputerVision.histogram."
                .to_string(),
            Some(json!({
                "data": data,
                "width": width,
                "height": height,
                "stride": width,
            })),
        ),
        None => (
            "Local histogram sketch: decode greyscale pixels onto this surface (data-grayscale-u8) before a live histogram."
                .to_string(),
            None,
        ),
    };
    invoke_dual(document, label, "ComputerVision.histogram", sketch, args);
}

/// `ComputerVision.equalize_hist` — histogram equalization on greyscale pixels.
pub(super) fn run_equalize_hist(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a picture surface before equalizing tones.",
    ) else {
        return;
    };
    let (sketch, args) = match grayscale_dims(&container) {
        Some((data, width, height)) => (
            "Local equalize sketch: greyscale pixels are present. Connect QualiaDB for ComputerVision.equalize_hist."
                .to_string(),
            Some(json!({
                "data": data,
                "width": width,
                "height": height,
                "stride": width,
            })),
        ),
        None => (
            "Local equalize sketch: decode greyscale pixels onto this surface (data-grayscale-u8) before live equalization."
                .to_string(),
            None,
        ),
    };
    invoke_dual(
        document,
        label,
        "ComputerVision.equalize_hist",
        sketch,
        args,
    );
}

/// `ComputerVision.rgb_to_gray` — RGB interleaved bytes → greyscale.
pub(super) fn run_rgb_to_gray(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a picture surface before converting to greyscale.",
    ) else {
        return;
    };
    let (sketch, args) = match rgb_dims(&container) {
        Some((data, width, height)) => (
            "Local rgb→gray sketch: RGB pixels are present. Connect QualiaDB for ComputerVision.rgb_to_gray."
                .to_string(),
            Some(json!({
                "data": data,
                "width": width,
                "height": height,
                "stride": width * 3,
            })),
        ),
        None => (
            "Local rgb→gray sketch: decode RGB pixels onto this surface (data-rgb-u8) before live conversion."
                .to_string(),
            None,
        ),
    };
    invoke_dual(document, label, "ComputerVision.rgb_to_gray", sketch, args);
}

/// `ComputerVision.dhash` — difference hash over greyscale pixels.
pub(super) fn run_dhash(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a picture surface before computing a difference hash.",
    ) else {
        return;
    };
    let (sketch, args) = match grayscale_dims(&container) {
        Some((data, width, height)) => (
            "Local dhash sketch: greyscale pixels are present. Connect QualiaDB for ComputerVision.dhash."
                .to_string(),
            Some(json!({
                "data": data,
                "width": width,
                "height": height,
                "stride": width,
            })),
        ),
        None => (
            "Local dhash sketch: decode greyscale pixels onto this surface (data-grayscale-u8) before a live hash."
                .to_string(),
            None,
        ),
    };
    invoke_dual(document, label, "ComputerVision.dhash", sketch, args);
}

/// `ComputerVision.hamming_distance` — distance between two u64 hashes.
pub(super) fn run_hamming_distance(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with two perceptual hashes before comparing.",
    ) else {
        return;
    };
    let a = container
        .get_attribute("data-hash-a")
        .or_else(|| container.get_attribute("data-dhash"))
        .and_then(|v| parse_u64_attr(&v));
    let b = container
        .get_attribute("data-hash-b")
        .or_else(|| container.get_attribute("data-ahash"))
        .and_then(|v| parse_u64_attr(&v));
    let (sketch, args) = match (a, b) {
        (Some(a), Some(b)) => {
            let local = (a ^ b).count_ones();
            (
                format!(
                    "Local hamming sketch: a={a:#x} b={b:#x} → {local} differing bits. Connect QualiaDB for ComputerVision.hamming_distance."
                ),
                Some(json!({ "a": a, "b": b })),
            )
        }
        _ => (
            "Local hamming sketch: set data-hash-a and data-hash-b (u64) on this surface before a live compare."
                .to_string(),
            None,
        ),
    };
    invoke_dual(
        document,
        label,
        "ComputerVision.hamming_distance",
        sketch,
        args,
    );
}

/// `ComputerVision.cosine_similarity` — similarity of two embedding vectors.
pub(super) fn run_cosine_similarity(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with two embedding vectors before comparing.",
    ) else {
        return;
    };
    let a = container
        .get_attribute("data-embedding-a")
        .map(|raw| parse_f64_csv(&raw))
        .filter(|v| !v.is_empty());
    let b = container
        .get_attribute("data-embedding-b")
        .map(|raw| parse_f64_csv(&raw))
        .filter(|v| !v.is_empty());
    let (sketch, args) = match (a, b) {
        (Some(a), Some(b)) if a.len() == b.len() => {
            let local = local_cosine(&a, &b);
            (
                format!(
                    "Local cosine sketch over {} dims ≈ {local:.4}. Connect QualiaDB for ComputerVision.cosine_similarity.",
                    a.len()
                ),
                Some(json!({ "a": a, "b": b })),
            )
        }
        (Some(a), Some(b)) => (
            format!(
                "Local cosine sketch blocked: vector lengths differ ({} vs {}).",
                a.len(),
                b.len()
            ),
            None,
        ),
        _ => (
            "Local cosine sketch: set data-embedding-a and data-embedding-b (CSV floats) before a live compare."
                .to_string(),
            None,
        ),
    };
    invoke_dual(
        document,
        label,
        "ComputerVision.cosine_similarity",
        sketch,
        args,
    );
}

fn local_cosine(a: &[f64], b: &[f64]) -> f64 {
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = (na.sqrt()) * (nb.sqrt());
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::{local_cosine, parse_u64_attr};

    #[test]
    fn parse_u64_accepts_hex_and_decimal() {
        assert_eq!(parse_u64_attr("42"), Some(42));
        assert_eq!(parse_u64_attr("0x10"), Some(16));
        assert_eq!(parse_u64_attr("0Xff"), Some(255));
        assert!(parse_u64_attr("nope").is_none());
    }

    #[test]
    fn local_cosine_identical_is_one() {
        let v = [1.0, 0.0, 0.0];
        assert!((local_cosine(&v, &v) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_cosine_orthogonal_is_zero() {
        let a = [1.0, 0.0];
        let b = [0.0, 1.0];
        assert!(local_cosine(&a, &b).abs() < 1e-9);
    }
}
