//! `medical` category — consolidated from crate-root modules (reorg).

#[cfg(not(target_arch = "wasm32"))]
pub mod comorbidity_eval;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_ingest;
