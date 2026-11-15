//! Deterministic 64-hex `.qac` publication, sidecar link validation, and Hann STFT
//! roundtrip verification helpers (`Q04`: `qac_production_publication`).
#![cfg(not(target_arch = "wasm32"))]

use crate::audio::{
    audio_sidecar_link::{compile_spectral_sheet_quin, format_sidecar_relpath},
    qac::{
        metadata::{OriginBasis, WindowKind},
        persistence::{QacArtifact, QacPublicationAdmission, QacPublishError},
        production::{
            produce_pcm, produce_stft, publish_current_signal, Error as ProductionError,
            ProductionOptions, StftProductionOptions,
        },
        signal::{
            self,
            stft::{StftDocument, StftWriteBuffers},
            writer::{PcmDocument, WriteBuffers},
            Limits as SignalLimits, Rate,
        },
        QacStorageOptions, QacStorageView, QAC_EXTENSION,
    },
};
use crate::q42::field_semantics as sem;
use crate::tensor::bake_pipeline::sigma_sheet_index_from_nquin;
use crate::NQuin;
use std::{fs, path::Path};

/// Canonical `spectral/audio/<64-hex>.qac` relative path length in bytes (15 + 64 + 4 = 83).
pub const CANONICAL_SIDECAR_RELPATH_LEN: usize = 83;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedHannStftPublication {
    pub artifact: QacArtifact,
    pub pcm_entry_index: u32,
    pub stft_entry_index: u32,
    pub pcm_records: u64,
    pub stft_frames: u64,
    pub fft_size: u32,
    pub hop: u32,
    pub frame_rate: Rate,
    pub window_kind: WindowKind,
    pub origin_basis: OriginBasis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedSidecarBinding {
    pub blake3: [u8; 32],
    pub relpath_bytes: [u8; CANONICAL_SIDECAR_RELPATH_LEN],
    pub sheet_index: u32,
    pub quin: NQuin,
}
impl VerifiedSidecarBinding {
    pub fn relpath_str(&self) -> &str {
        core::str::from_utf8(&self.relpath_bytes).unwrap_or("")
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum VerificationError {
    ZeroDigest,
    InvalidRelpath,
    ForbiddenLegacyExtension,
    DigestMismatch,
    FilenameMismatch,
    AdmissionMismatch,
    Storage(crate::audio::qac::QacStorageError),
    Signal(signal::Error),
    PublishConflictExpected,
    Production(String),
}

/// Reject all-zero 32-byte digests and any `.q4au` suffix; format and verify the
/// canonical `spectral/audio/<64 lowercase hex>.qac` relative path and `NQuin` binding.
pub fn verify_sidecar_link_binding(
    subject_hash: u64,
    sheet_index: u32,
    digest: &[u8; 32],
    relpath_candidate: &str,
) -> Result<VerifiedSidecarBinding, VerificationError> {
    if digest.iter().all(|b| *b == 0) {
        return Err(VerificationError::ZeroDigest);
    }
    if relpath_candidate.ends_with(".q4au") || relpath_candidate.contains(".q4au") {
        return Err(VerificationError::ForbiddenLegacyExtension);
    }
    let mut relpath_bytes = [0u8; CANONICAL_SIDECAR_RELPATH_LEN];
    if format_sidecar_relpath(digest, &mut relpath_bytes) != CANONICAL_SIDECAR_RELPATH_LEN {
        return Err(VerificationError::InvalidRelpath);
    }
    let canonical =
        core::str::from_utf8(&relpath_bytes).map_err(|_| VerificationError::InvalidRelpath)?;
    if relpath_candidate != canonical
        || !canonical.starts_with("spectral/audio/")
        || !canonical.ends_with(".qac")
    {
        return Err(VerificationError::InvalidRelpath);
    }
    let hex_part = &canonical["spectral/audio/".len()..canonical.len() - ".qac".len()];
    if hex_part.len() != 64
        || !hex_part
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(VerificationError::InvalidRelpath);
    }
    let quin = compile_spectral_sheet_quin(subject_hash, sheet_index);
    if sigma_sheet_index_from_nquin(&quin) != Some(sheet_index) {
        return Err(VerificationError::InvalidRelpath);
    }
    Ok(VerifiedSidecarBinding { blake3: *digest, relpath_bytes, sheet_index, quin })
}

/// Verify that a published `QacArtifact` matches the exact full-file BLAKE3 digest of
/// `expected_bytes`, resides at `spectral/audio/<64-hex>.qac`, and re-publishes idempotently.
pub fn verify_published_artifact_determinism(
    root: &Path,
    artifact: &QacArtifact,
    expected_bytes: &[u8],
    scratch: &mut sem::Scratch<'_>,
    options: ProductionOptions,
) -> Result<VerifiedSidecarBinding, VerificationError> {
    if artifact.blake3.iter().all(|b| *b == 0) {
        return Err(VerificationError::ZeroDigest);
    }
    let actual_digest = *blake3::hash(expected_bytes).as_bytes();
    if artifact.blake3 != actual_digest
        || artifact.byte_length != expected_bytes.len() as u64
        || artifact.admission != QacPublicationAdmission::CurrentStructure
    {
        return Err(VerificationError::DigestMismatch);
    }
    if fs::read(&artifact.path).map_err(|_| VerificationError::DigestMismatch)? != expected_bytes {
        return Err(VerificationError::DigestMismatch);
    }
    let ext = artifact.path.extension().and_then(|s| s.to_str());
    if ext == Some("q4au") {
        return Err(VerificationError::ForbiddenLegacyExtension);
    }
    if ext != Some(QAC_EXTENSION) {
        return Err(VerificationError::FilenameMismatch);
    }
    let expected_name = format!("{}.qac", blake3::Hash::from(actual_digest).to_hex());
    if artifact.path.file_name().and_then(|s| s.to_str()) != Some(expected_name.as_str()) {
        return Err(VerificationError::FilenameMismatch);
    }
    let mut rel_buf = [0u8; CANONICAL_SIDECAR_RELPATH_LEN];
    if format_sidecar_relpath(&artifact.blake3, &mut rel_buf) != CANONICAL_SIDECAR_RELPATH_LEN {
        return Err(VerificationError::InvalidRelpath);
    }
    let rel_str = core::str::from_utf8(&rel_buf).map_err(|_| VerificationError::InvalidRelpath)?;
    if !artifact.path.ends_with(rel_str) {
        return Err(VerificationError::FilenameMismatch);
    }
    let republished = publish_current_signal(root, expected_bytes, scratch, options)
        .map_err(|e| VerificationError::Production(e.to_string()))?;
    if &republished != artifact {
        return Err(VerificationError::AdmissionMismatch);
    }
    verify_sidecar_link_binding(1, 0, &artifact.blake3, rel_str)
}

/// Produce a Hann-windowed complex STFT + retained f32 PCM `.qac` artifact, verify deterministic
/// 64-hex publication, open via `QacStorageView::open`, and admit both the Mode `0x0101` STFT
/// entry and the retained Mode `0x0100` PCM source entry.
pub fn verify_hann_stft_production_roundtrip(
    root: &Path,
    document: &StftDocument<'_>,
    buffers: StftWriteBuffers<'_>,
    scratch: &mut sem::Scratch<'_>,
    final_bytes: &mut [u8],
    options: StftProductionOptions,
) -> Result<VerifiedHannStftPublication, VerificationError> {
    let artifact = produce_stft(root, document, buffers, scratch, final_bytes, options)
        .map_err(|e| VerificationError::Production(e.to_string()))?;
    let published = &final_bytes[..artifact.byte_length as usize];
    verify_published_artifact_determinism(
        root,
        &artifact,
        published,
        scratch,
        ProductionOptions {
            publication_max_bytes: options.publication_max_bytes,
            ..ProductionOptions::default()
        },
    )?;
    let storage = QacStorageView::open(
        published,
        QacStorageOptions { envelope_limits: options.writer.envelope },
    )
    .map_err(VerificationError::Storage)?;
    let structure = storage.structure();
    let pcm_idx = document.pcm_entry_index();
    let stft_idx = document.stft_entry_index();
    let pcm_view = signal::admit(structure, pcm_idx, scratch, options.writer.shared, options.writer.signal)
        .map_err(VerificationError::Signal)?;
    let stft_view = signal::admit(structure, stft_idx, scratch, options.writer.shared, options.writer.signal)
        .map_err(VerificationError::Signal)?;
    if pcm_view.mode() != 0x0100 || stft_view.mode() != 0x0101 {
        return Err(VerificationError::AdmissionMismatch);
    }
    let tx = stft_view.transform().ok_or(VerificationError::AdmissionMismatch)?;
    if tx.window_kind() != WindowKind::HannSymmetricF32
        || tx.origin_basis() != OriginBasis::SourceEntryFirstSample
        || tx.origin_samples != 0
    {
        return Err(VerificationError::AdmissionMismatch);
    }
    let win_le = stft_view.window_coefficients_le().ok_or(VerificationError::AdmissionMismatch)?;
    if win_le.len() != (document.fft_size as usize) * 4 {
        return Err(VerificationError::AdmissionMismatch);
    }
    let expected_rate = document.timing.samples.divided_by(document.hop).map_err(VerificationError::Signal)?;
    let stft_timing = stft_view.timing().ok_or(VerificationError::AdmissionMismatch)?;
    if stft_timing.samples != expected_rate {
        return Err(VerificationError::AdmissionMismatch);
    }
    Ok(VerifiedHannStftPublication {
        artifact,
        pcm_entry_index: pcm_idx,
        stft_entry_index: stft_idx,
        pcm_records: pcm_view.records(),
        stft_frames: stft_view.records(),
        fft_size: tx.fft_size,
        hop: tx.hop,
        frame_rate: stft_timing.samples,
        window_kind: tx.window_kind(),
        origin_basis: tx.origin_basis(),
    })
}

/// Verify that `produce_pcm` publishes deterministically and is idempotent on re-publish.
pub fn verify_pcm_production_roundtrip(
    root: &Path,
    document: &PcmDocument<'_>,
    buffers: WriteBuffers<'_>,
    scratch: &mut sem::Scratch<'_>,
    final_bytes: &mut [u8],
    options: ProductionOptions,
) -> Result<(QacArtifact, u64), VerificationError> {
    let artifact = produce_pcm(root, document, buffers, scratch, final_bytes, options)
        .map_err(|e| VerificationError::Production(e.to_string()))?;
    let published = &final_bytes[..artifact.byte_length as usize];
    verify_published_artifact_determinism(root, &artifact, published, scratch, options)?;
    let storage = QacStorageView::open(
        published,
        QacStorageOptions { envelope_limits: options.writer.envelope },
    )
    .map_err(VerificationError::Storage)?;
    let entry_idx = if document.original.is_some() { 2 } else { 1 };
    let pcm_view = signal::admit(
        storage.structure(),
        entry_idx,
        scratch,
        options.writer.shared,
        SignalLimits::default(),
    )
    .map_err(VerificationError::Signal)?;
    if pcm_view.mode() != 0x0100 {
        return Err(VerificationError::AdmissionMismatch);
    }
    Ok((artifact, pcm_view.records()))
}

/// Verify that conflicting bytes at an already-published `<64-hex>.qac` path fail closed
/// with `QacPublishError::Conflict` without clobbering the existing file on disk.
pub fn verify_noclobber_conflict_fails_closed(
    root: &Path,
    valid_bytes: &[u8],
    scratch: &mut sem::Scratch<'_>,
    options: ProductionOptions,
) -> Result<(), VerificationError> {
    let artifact = publish_current_signal(root, valid_bytes, scratch, options)
        .map_err(|e| VerificationError::Production(e.to_string()))?;
    let mut tampered = valid_bytes.to_vec();
    if let Some(last) = tampered.last_mut() {
        *last ^= 0xff;
    }
    fs::write(&artifact.path, &tampered).map_err(|_| VerificationError::DigestMismatch)?;
    match publish_current_signal(root, valid_bytes, scratch, options) {
        Err(ProductionError::Publication(QacPublishError::Conflict)) => {}
        _ => return Err(VerificationError::PublishConflictExpected),
    }
    if fs::read(&artifact.path).map_err(|_| VerificationError::DigestMismatch)? != tampered {
        return Err(VerificationError::PublishConflictExpected);
    }
    fs::write(&artifact.path, valid_bytes).map_err(|_| VerificationError::DigestMismatch)?;
    let restored = publish_current_signal(root, valid_bytes, scratch, options)
        .map_err(|e| VerificationError::Production(e.to_string()))?;
    if restored != artifact {
        return Err(VerificationError::AdmissionMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::qac::signal::{
        stft::StftQuantity,
        writer::{PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation},
        Sample, Timing,
    };
    use etalon_signal::audio::{fill_hann_window_into, stft_frame_into};

    fn sample_pcm_doc<'a>(samples: &'a [f32]) -> PcmDocument<'a> {
        let r = Rate::new(48_000, 1).unwrap();
        PcmDocument {
            samples: PcmSamples::F32(samples),
            timing: Timing { scale: 0, epoch_seconds: 0, epoch_nanoseconds: 0, ticks: r, samples: r, start_tick: 0 },
            stream_id: 1,
            channels: 1,
            channel_order: "mic-0",
            purpose: "deterministic-qac-verify",
            quantity: PcmQuantity::DigitalSample,
            relation: SignalRelation::DigitalUnrelatedToPressure,
            units_per_stored_sample: 1.0,
            stored_offset: 0.0,
            provenance: Provenance {
                role: ProvenanceRole::Measured,
                algorithm: None,
                algorithm_version: None,
                instrument_uri: Some("urn:qualia:mic:0"),
                lineage: None,
            },
            original: None,
        }
    }

    fn sample_stft_doc<'a>(samples: &'a [f32], fft_size: u32, hop: u32, rate: Rate) -> StftDocument<'a> {
        let ticks = Rate::new(96_000, 1).unwrap();
        StftDocument {
            samples: PcmSamples::F32(samples),
            timing: Timing { scale: 1, epoch_seconds: 10, epoch_nanoseconds: 500, ticks, samples: rate, start_tick: -7 },
            stream_id: 3,
            channels: 1,
            channel_order: "mic-0",
            fft_size,
            hop,
            pcm_purpose: "retained-pcm-source",
            stft_purpose: "hann-complex-stft",
            quantity: StftQuantity::DigitalSample,
            relation: SignalRelation::DigitalUnrelatedToPressure,
            pcm_provenance: Provenance {
                role: ProvenanceRole::Measured,
                algorithm: None,
                algorithm_version: None,
                instrument_uri: Some("urn:qualia:mic:0"),
                lineage: None,
            },
            stft_provenance: Provenance {
                role: ProvenanceRole::Derived,
                algorithm: Some("urn:etalon:signal:hann-stft-f32"),
                algorithm_version: Some("1"),
                instrument_uri: None,
                lineage: Some("symmetric-hann-radix2-no-padding"),
            },
            original: Some(b"raw-capture-header\0\xfe"),
        }
    }

    fn run_stft(root: &Path, doc: &StftDocument<'_>) -> (VerifiedHannStftPublication, Vec<u8>) {
        let (mut mono, mut fft, mut win) = ([0.0f32; 16], [0.0f32; 32], [0.0f32; 16]);
        let (mut spectra, mut metadata, mut payload) = ([0u8; 4096], [0u8; 8192], [0u8; 4096]);
        let mut file = [0u8; 16384];
        let mut out = vec![0xa5u8; 16384];
        let mut m = [sem::NodeState::default(); 32];
        let mut e = m;
        let res = verify_hann_stft_production_roundtrip(
            root,
            doc,
            StftWriteBuffers {
                mono: &mut mono,
                fft: &mut fft,
                window: &mut win,
                spectra: &mut spectra,
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file,
            },
            &mut sem::Scratch { metadata: &mut m, entries: &mut e },
            &mut out,
            StftProductionOptions::default(),
        )
        .unwrap();
        out.truncate(res.artifact.byte_length as usize);
        (res, out)
    }

    #[test]
    fn hann_stft_roundtrip_and_large_prime_rate_divided_by_succeed() {
        let root = tempfile::tempdir().unwrap();
        let samples = [0.25f32, -0.5, 0.75, -0.25, 0.5, -0.75, 0.125];
        let large_rate = Rate::new(1_000_000_007 * 999_983, 999_999_999_989).unwrap();
        let doc = sample_stft_doc(&samples, 4, 1_000_000_007, large_rate);
        let (verified, published) = run_stft(root.path(), &doc);
        assert_eq!(verified.pcm_entry_index, 2);
        assert_eq!(verified.stft_entry_index, 3);
        assert_eq!(verified.pcm_records, 7);
        assert_eq!(verified.stft_frames, 1);
        assert_eq!(verified.window_kind, WindowKind::HannSymmetricF32);
        assert_eq!(verified.origin_basis, OriginBasis::SourceEntryFirstSample);
        assert_eq!(verified.frame_rate, Rate { numerator: 999_983, denominator: 999_999_999_989 });

        let storage = QacStorageView::open(&published, QacStorageOptions::default()).unwrap();
        let mut m = [sem::NodeState::default(); 32];
        let mut e = m;
        let stft_view = signal::admit(
            storage.structure(),
            verified.stft_entry_index,
            &mut sem::Scratch { metadata: &mut m, entries: &mut e },
            sem::Limits::default(),
            SignalLimits::default(),
        )
        .unwrap();
        let mut expected_win = [0.0f32; 4];
        fill_hann_window_into(&mut expected_win).unwrap();
        let win_bytes = stft_view.window_coefficients_le().unwrap();
        for (i, chunk) in win_bytes.chunks_exact(4).enumerate() {
            assert_eq!(chunk, &expected_win[i].to_le_bytes());
        }
        let mut expected_spec = [0.0f32; 8];
        stft_frame_into(&samples[..4], &mut expected_spec).unwrap();
        for bin in 0..4usize {
            let Sample::ComplexF32 { real, imag } = stft_view.sample(0, 0, bin as u32).unwrap() else {
                panic!()
            };
            assert_eq!(real.to_bits(), expected_spec[bin * 2].to_bits());
            assert_eq!(imag.to_bits(), expected_spec[bin * 2 + 1].to_bits());
        }
    }

    #[test]
    fn distinct_stft_inputs_sharing_64bin_preview_produce_distinct_64hex_qac_and_reject_q4au() {
        let root = tempfile::tempdir().unwrap();
        let s1 = [0.5f32, -0.5, 0.5, -0.5, 0.25, -0.25];
        let s2 = [0.5f32, -0.5, 0.5, -0.5, 0.25, -0.25, 0.75, -0.75];
        let (rate_a, rate_b) = (Rate::new(48_000, 1).unwrap(), Rate::new(96_000, 1).unwrap());
        let docs = [
            sample_stft_doc(&s1, 4, 2, rate_a),
            sample_stft_doc(&s2, 4, 2, rate_a),
            sample_stft_doc(&s1, 4, 1, rate_a),
            sample_stft_doc(&s1, 4, 2, rate_b),
        ];
        let mut digests = Vec::new();
        for (idx, d) in docs.iter().enumerate() {
            let (res, _) = run_stft(root.path(), d);
            let mut rel = [0u8; CANONICAL_SIDECAR_RELPATH_LEN];
            let n = format_sidecar_relpath(&res.artifact.blake3, &mut rel);
            let rel_str = core::str::from_utf8(&rel[..n]).unwrap();
            let binding = verify_sidecar_link_binding(0x42, idx as u32, &res.artifact.blake3, rel_str).unwrap();
            assert_eq!(binding.sheet_index, idx as u32);
            assert!(binding.relpath_str().ends_with(".qac"));
            digests.push(res.artifact.blake3);
        }
        for i in 0..digests.len() {
            for j in (i + 1)..digests.len() {
                assert_ne!(digests[i], digests[j]);
            }
        }
        assert_eq!(
            verify_sidecar_link_binding(1, 0, &[0u8; 32], "spectral/audio/00.qac").unwrap_err(),
            VerificationError::ZeroDigest
        );
        let legacy = format!("spectral/audio/{}.q4au", blake3::Hash::from(digests[0]).to_hex());
        assert_eq!(
            verify_sidecar_link_binding(1, 0, &digests[0], &legacy).unwrap_err(),
            VerificationError::ForbiddenLegacyExtension
        );
    }

    #[test]
    fn pcm_publication_and_noclobber_conflict_fail_closed_without_clobbering() {
        let root = tempfile::tempdir().unwrap();
        let samples = [0.125f32, -0.25, 0.5, -0.75];
        let doc = sample_pcm_doc(&samples);
        let (mut metadata, mut payload, mut file) = ([0u8; 4096], [0u8; 4096], [0u8; 16384]);
        let mut out = vec![0xa5u8; 16384];
        let mut m = [sem::NodeState::default(); 32];
        let mut e = m;
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let (artifact, records) = verify_pcm_production_roundtrip(
            root.path(),
            &doc,
            WriteBuffers { metadata: &mut metadata, payload: &mut payload, file: &mut file },
            &mut scratch,
            &mut out,
            ProductionOptions::default(),
        )
        .unwrap();
        assert_eq!(records, 4);
        let valid_bytes = &out[..artifact.byte_length as usize];
        verify_noclobber_conflict_fails_closed(
            root.path(),
            valid_bytes,
            &mut scratch,
            ProductionOptions::default(),
        )
        .unwrap();
    }
}
