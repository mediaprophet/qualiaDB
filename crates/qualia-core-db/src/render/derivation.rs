//! Decoupled Derivation Job (P2)
//!
//! Converts an original source asset (e.g., an OBJ or GLB file) into a `.10d` hypermedia
//! container carrying the provenance sidecar (the source bytes + metadata) bound within it.
//! This fulfills the "context is the asset" mandate, ensuring native geometry and provenance
//! are inseparable.

use crate::container_10d::provenance_section::ProvenanceSidecar;
use crate::render::assets::import_asset;
use crate::render::compile_10d::{compile_mesh_to_10d_with_provenance, Compile10dError};

/// Failure modes for the derivation job.
#[derive(Debug, PartialEq, Eq)]
pub enum DerivationError {
    /// Failed to parse or process the source geometry.
    ImportFailed(String),
    /// Failed to compile the `.10d` container or sidecar.
    CompilationFailed(Compile10dError),
}

impl std::fmt::Display for DerivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ImportFailed(e) => write!(f, "Derivation import failed: {e}"),
            Self::CompilationFailed(e) => write!(f, "Derivation compilation failed: {e}"),
        }
    }
}
impl std::error::Error for DerivationError {}

/// Run the decoupled derivation job on an original source asset.
///
/// Takes the raw source bytes, imports them into a mesh, wraps the source bytes and
/// metadata into a `ProvenanceSidecar`, and compiles it all into a sealed `.10d` container.
pub fn run_derivation_job(
    source_bytes: &[u8],
    format_hint: Option<&str>,
    media_type: &str,
    licence: &str,
    vc_payload: Option<&[u8]>,
) -> Result<Vec<u8>, DerivationError> {
    let mesh = import_asset(source_bytes, format_hint)
        .map_err(|e| DerivationError::ImportFailed(e.to_string()))?;

    let mut sidecar = ProvenanceSidecar::new(source_bytes, media_type, licence);
    if let Some(vc_bytes) = vc_payload {
        sidecar = sidecar.with_vc(vc_bytes);
    }

    compile_mesh_to_10d_with_provenance(&mesh, Some(&sidecar))
        .map_err(DerivationError::CompilationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::header::Container10dHeader;
    use crate::container_10d::provenance_section::{
        decode_provenance_section, validate_provenance,
    };
    use crate::container_10d::section::{parse_section_table, SectionType};

    const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    #[test]
    fn derivation_job_produces_valid_container() {
        let out = run_derivation_job(TRI_OBJ, Some("obj"), "text/plain", "CC0", None).unwrap();

        let header = Container10dHeader::parse(&out).unwrap();
        let descs = parse_section_table(&out, &header).unwrap();

        // Find provenance sidecar
        let prov = descs
            .iter()
            .find(|d| d.section_type == SectionType::ProvenanceSidecar as u8)
            .expect("provenance section generated");

        let payload = &out[prov.byte_offset as usize..][..prov.byte_length as usize];
        let view = decode_provenance_section(payload).unwrap();

        validate_provenance(&view).unwrap();
        assert_eq!(view.licence(), "CC0");
        assert_eq!(view.source_bytes(), TRI_OBJ);
    }
}
