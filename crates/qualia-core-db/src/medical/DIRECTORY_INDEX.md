---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# medical Index

## Functionality Overview
Comprehensive index of functionality for `medical`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `comorbidity_eval.rs`
  - `enum ComorbidityStatus`
  - `struct ComorbidityVerdict`
  - `enum ComorbidityError`
  - `fn nested_claim_fingerprint`
  - `fn is_nested_subject`
  - `fn decode_severity_milli`
  - `fn encode_severity_object`
  - `fn compile_exacerbation_quins`
  - `fn condition_intersects_organ`
  - `fn eval_comorbidity`
  - `fn patient_ctx`
  - `fn compile_demo_graph`
  - `fn nested_fingerprint_sets_msb`
  - `fn eval_finds_compounded_diabetes_neuropathy_risk`
  - `fn zero_heap_eval_comorbidity`
- 📄 `dicom.rs`
  - `enum DicomError`
  - `impl std`
  - `fn fmt`
  - `struct DicomMetadata`
  - `struct DicomPixelSlice`
  - `struct DicomSplitPayload`
  - `struct DicomPlacement`
  - `struct DicomOverlaySpec`
  - `struct DicomOrganMapFile`
  - `struct DicomTagMatcher`
  - `enum TransferSyntax`
  - `fn find_dataset_offset`
  - `fn read_u16_le`
  - `fn read_u32_le`
  - `fn vr_is_long`
  - *(...and 49 more)*
- 📄 `dicom_ingest.rs`
  - `struct DicomSeriesRecord`
  - `struct IngestJob`
  - `struct JobSlot`
  - `impl JobSlot`
  - `struct SyncSeriesRegistry`
  - `enum DicomIngestError`
  - `impl std`
  - `fn fmt`
  - `struct DicomBlobStore`
  - `impl DicomBlobStore`
  - `fn open`
  - `fn append_pixels`
  - `fn path_mmap`
  - `struct DicomBlobReader`
  - `impl DicomBlobReader`
  - *(...and 22 more)*
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
