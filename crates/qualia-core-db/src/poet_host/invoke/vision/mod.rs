//! Computer vision invoke seams.
//!
//! Exposes `specialized_libs::computer_vision` through VibeScript invoke IDs
//! in the `ComputerVision.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ahash;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cv;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ahash::ahash;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cv::{
    canny_edges, cosine_similarity, dhash, equalize_hist, gaussian_blur, hamming_distance,
    histogram, rgb_to_gray, sobel_magnitude,
};

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ahash(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn gaussian_blur(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn sobel_magnitude(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn canny_edges(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn histogram(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn equalize_hist(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn rgb_to_gray(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn dhash(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hamming_distance(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cosine_similarity(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}
