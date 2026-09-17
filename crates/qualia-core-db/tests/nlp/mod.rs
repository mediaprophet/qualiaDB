//! NLP-004 evaluation modules + NLP-005 corpus loaders.
//! Declared from `tests/nlp_suite.rs`.
//! Cargo does not auto-discover nested files under `tests/nlp/`.

pub mod corpus;
pub mod gate;
pub mod json_lite;
pub mod manifest;
pub mod receipt;
pub mod regression;
pub mod score;
