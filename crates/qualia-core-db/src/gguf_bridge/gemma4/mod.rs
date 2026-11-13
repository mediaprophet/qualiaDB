//! Architecture-owned Gemma4 GGUF execution; retained text token and per-layer KV.
mod attention;
mod bridge;
mod ffn;
mod metadata;
mod plan;
mod ple;
mod primitives;
mod session;
#[cfg(test)]
mod tests;
pub(crate) use metadata::Profile;
pub(crate) use session::Session;

#[cfg(test)]
mod metadata_tests;
