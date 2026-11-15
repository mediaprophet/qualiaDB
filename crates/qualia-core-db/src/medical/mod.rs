//! `medical` category (reorg).

pub mod comorbidity_eval;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_part10;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_geometry;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_ingest;
#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_source;

#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_dataset;

#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_frames;

#[cfg(not(target_arch = "wasm32"))]
pub mod dicom_codecs;

#[cfg(not(target_arch = "wasm32"))]
pub mod reconstruction;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod conformance_tests;
