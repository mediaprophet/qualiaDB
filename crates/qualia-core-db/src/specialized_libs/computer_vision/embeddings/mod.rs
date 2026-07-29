//! Local-first visual embedding proxies for decentralized CBIR / near-duplicate search.
//!
//! # Honesty
//!
//! This is **not** foundation CLIP (or any trained open-vocab embedder). It is a
//! pure-Rust, zero-model-weight **local CBIR proxy**:
//! - [`perceptual_hash_u64`] — aHash / dHash structure fingerprints
//! - [`color_hist_embed`] — fixed RGB joint histogram, L2-normalized into a caller buffer
//! - [`embed_distance`] — Hamming (hashes) + cosine (float embeds)
//!
//! Place ONNX CLIP (or ResNet) under `vendor/vision/embeddings/` when product needs
//! semantic retrieval; keep these APIs as the cheap, always-on pre-filter.

pub mod color_hist_embed;
pub mod embed_distance;
pub mod perceptual_hash_u64;

pub use color_hist_embed::{color_hist_embed_rgb, COLOR_HIST_BINS, COLOR_HIST_EMBED_DIM};
pub use embed_distance::{cosine_distance, cosine_similarity, hamming_distance_u64};
pub use perceptual_hash_u64::{ahash_u64, dhash_u64, AHASH_SIDE, DHASH_HEIGHT, DHASH_WIDTH};
