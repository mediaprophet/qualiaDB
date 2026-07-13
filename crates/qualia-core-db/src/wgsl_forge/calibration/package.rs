//! W10 calibration — stage 5: provenance + packaging.
//!
//! A certified artifact is framed with a CBOR provenance header (corpus hash, engine version, gate
//! numbers) so the engine can refuse an artifact that wasn't produced + certified by the forge.
//! Provenance rides as CBOR — the project's canonical payload substrate (see the memory note
//! `feedback-cbor-ld-payloads-not-adhoc-json`), not ad-hoc JSON.

#![cfg(not(target_arch = "wasm32"))]

use super::{ArtifactKind, CalibrationError};

/// Frame magic — `QCAL` + version.
pub const FRAME_MAGIC: [u8; 8] = *b"QCAL0001";

/// Certification provenance for a calibration artifact.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    pub kind: ArtifactKind,
    /// FNV-1a content hash of the calibration corpus.
    pub corpus_hash: u64,
    pub corpus_docs: usize,
    /// Engine/crate version that produced + certified this artifact (`CARGO_PKG_VERSION`).
    pub engine_version: String,
    pub ref_ppl: f64,
    pub cand_ppl: f64,
    pub delta_ppl: f64,
    pub passed: bool,
}

impl Provenance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: ArtifactKind,
        corpus_hash: u64,
        corpus_docs: usize,
        ref_ppl: f64,
        cand_ppl: f64,
        delta_ppl: f64,
        passed: bool,
    ) -> Self {
        Self {
            kind,
            corpus_hash,
            corpus_docs,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            ref_ppl,
            cand_ppl,
            delta_ppl,
            passed,
        }
    }

    /// Encode as CBOR (infallible for this schema; `Vec` writer never errors).
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(self, &mut buf);
        buf
    }

    /// Decode from CBOR; `None` on malformed bytes / schema mismatch.
    pub fn from_cbor(bytes: &[u8]) -> Option<Self> {
        ciborium::from_reader(bytes).ok()
    }
}

/// Frame `artifact` bytes behind the CBOR provenance header:
/// `[ MAGIC(8) | prov_len:u32 LE | CBOR(provenance) | artifact bytes ]`.
pub fn frame_artifact(artifact: &[u8], prov: &Provenance) -> Vec<u8> {
    let cbor = prov.to_cbor();
    let mut out = Vec::with_capacity(8 + 4 + cbor.len() + artifact.len());
    out.extend_from_slice(&FRAME_MAGIC);
    out.extend_from_slice(&(cbor.len() as u32).to_le_bytes());
    out.extend_from_slice(&cbor);
    out.extend_from_slice(artifact);
    out
}

/// Parse a framed artifact → `(provenance, artifact_bytes)`. The engine calls this before adopting
/// an artifact so an unframed / corrupt / unparseable-provenance blob is rejected (fail-closed).
pub fn parse_frame(bytes: &[u8]) -> Result<(Provenance, &[u8]), CalibrationError> {
    if bytes.len() < 12 || bytes[..8] != FRAME_MAGIC {
        return Err(CalibrationError::PackageFailed("bad frame magic".into()));
    }
    let prov_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let start = 12usize;
    let end = start
        .checked_add(prov_len)
        .filter(|&e| e <= bytes.len())
        .ok_or_else(|| CalibrationError::PackageFailed("provenance length out of range".into()))?;
    let prov = Provenance::from_cbor(&bytes[start..end])
        .ok_or_else(|| CalibrationError::PackageFailed("provenance CBOR unparseable".into()))?;
    Ok((prov, &bytes[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prov() -> Provenance {
        Provenance::new(
            ArtifactKind::AwqScales,
            0xDEAD_BEEF,
            12,
            8.5,
            8.7,
            0.0235,
            true,
        )
    }

    #[test]
    fn provenance_cbor_round_trips() {
        let p = sample_prov();
        let back = Provenance::from_cbor(&p.to_cbor()).expect("round-trip");
        assert_eq!(back, p);
        assert!(Provenance::from_cbor(&[0xFF, 0x00, 0x13]).is_none());
    }

    #[test]
    fn frame_round_trips_and_preserves_artifact() {
        let p = sample_prov();
        let artifact = b"\x00\x01\x02the-real-artifact-bytes\xff";
        let framed = frame_artifact(artifact, &p);
        let (prov, body) = parse_frame(&framed).expect("parse");
        assert_eq!(prov, p);
        assert_eq!(body, artifact);
    }

    #[test]
    fn parse_frame_rejects_garbage_and_truncation() {
        assert!(parse_frame(b"not-a-frame").is_err());
        assert!(parse_frame(&[]).is_err());
        let mut framed = frame_artifact(b"x", &sample_prov());
        // Corrupt the declared provenance length to overrun the buffer.
        framed[8] = 0xFF;
        framed[9] = 0xFF;
        assert!(parse_frame(&framed).is_err());
    }
}
