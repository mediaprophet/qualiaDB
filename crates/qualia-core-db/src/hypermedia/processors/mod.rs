//! **Content processors** — ingest *derives* searchability. Each processor
//! turns an asset's bytes into the derived, searchable representations +
//! descriptor facets (+ flags) that fold into its container, so the *original*
//! becomes findable by meaning.
//!
//! The framework itself — the [`Processor`](super::Processor) trait,
//! [`ProcessorOutput`](super::ProcessorOutput), and the model-free
//! [`TextProcessor`](super::TextProcessor) — lives in the parent module. This
//! submodule holds the heavier, self-contained processors and the dispatcher
//! that picks one by media type:
//!
//! - [`ImageProcessor`] — EXIF/PNG metadata → timeline + map facets (model-free).
//! - [`WavProcessor`] — WAV → duration + dominant-frequency, via the project's STFT.
//!
//! **Plug-in points (honest gaps, not stubs):** *what an image depicts* /
//! OCR needs a vision model, and a *transcript* needs an ASR model. Both are
//! new `Processor` implementations to register here when the `qualia-vision` /
//! `qualia-audio` model engines exist — the dispatcher already routes by media
//! type, so they slot in without touching callers.

pub mod audio;
pub mod image;

pub use audio::{AudioSpectralSummary, WavProcessor};
pub use image::{ImageMetadata, ImageProcessor};

use super::{Processor, TextProcessor};

/// Pick the processor that best handles `media_type`, or `None` if no
/// registered processor claims it (the caller can fall back to storing the
/// asset with no derived facets). Order matters only where `handles` overlaps;
/// here the media-type families are disjoint.
///
/// Returned boxed so heterogeneous processors share one call site; ingest is
/// not a hot path (it runs once per asset, off the render/query loops), so the
/// single allocation is acceptable.
pub fn processor_for(media_type: &str) -> Option<Box<dyn Processor>> {
    let candidates: [Box<dyn Processor>; 3] = [
        Box::new(ImageProcessor),
        Box::new(WavProcessor),
        Box::new(TextProcessor::default()),
    ];
    candidates.into_iter().find(|p| p.handles(media_type))
}

/// The media types a registered processor can derive searchability from — for
/// callers that want to advertise what ingest understands.
pub fn supported_media_types() -> &'static [&'static str] {
    &[
        "image/jpeg",
        "image/jpg",
        "image/png",
        "audio/wav",
        "audio/x-wav",
        "audio/wave",
        "text/*",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_routes_by_media_type() {
        assert!(processor_for("image/jpeg").unwrap().handles("image/jpeg"));
        assert!(processor_for("audio/wav").unwrap().handles("audio/wav"));
        assert!(processor_for("text/markdown")
            .unwrap()
            .handles("text/markdown"));
        // An unknown binary type has no registered processor.
        assert!(processor_for("application/octet-stream").is_none());
    }
}
