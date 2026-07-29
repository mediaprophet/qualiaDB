//! F4 — rights / attestation barrier for vision `.10d` browse and load.
//!
//! Fail-closed for **citable** use: CRC must verify and ProvenanceSidecar must
//! be present and pass `validate_provenance`. Browse-only may allow unattested
//! recon for local development when explicitly opted in.

use qualia_core_db::container_10d::{
    header::Container10dHeader,
    integrity::verify_whole_file_crc32c,
    parse_section_table,
    provenance_section::{decode_provenance_section, validate_provenance},
    section::SectionType,
};
use serde::Serialize;

/// Access mode for vision 10D assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Vision10dAccess {
    /// Local browse / debug: CRC required; provenance optional.
    BrowseAllowUnattested,
    /// Product / citable: CRC + valid ProvenanceSidecar required.
    CitableRequireProvenance,
}

/// Barrier outcome (F4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Vision10dBarrier {
    Permit,
    Deny { reason: &'static str },
}

/// Evaluate rights barrier on sealed container bytes.
pub fn evaluate_vision_10d_barrier(bytes: &[u8], access: Vision10dAccess) -> Vision10dBarrier {
    let mut bytes_mut = bytes.to_vec();
    if verify_whole_file_crc32c(&mut bytes_mut).is_err() {
        return Vision10dBarrier::Deny {
            reason: "crc_failed",
        };
    }
    let Ok(header) = Container10dHeader::parse(&bytes_mut) else {
        return Vision10dBarrier::Deny {
            reason: "bad_header",
        };
    };
    let Ok(descs) = parse_section_table(&bytes_mut, &header) else {
        return Vision10dBarrier::Deny {
            reason: "bad_section_table",
        };
    };

    let mut has_mesh = false;
    let mut prov_payload: Option<&[u8]> = None;
    for d in descs.iter() {
        match d.typ() {
            Some(SectionType::QuantizedMesh) => has_mesh = true,
            Some(SectionType::ProvenanceSidecar) => {
                let start = d.byte_offset as usize;
                let end = start.saturating_add(d.byte_length as usize);
                prov_payload = bytes_mut.get(start..end);
            }
            _ => {}
        }
    }
    if !has_mesh {
        return Vision10dBarrier::Deny {
            reason: "no_mesh_section",
        };
    }

    match access {
        Vision10dAccess::BrowseAllowUnattested => Vision10dBarrier::Permit,
        Vision10dAccess::CitableRequireProvenance => match prov_payload {
            None => Vision10dBarrier::Deny {
                reason: "missing_provenance",
            },
            Some(payload) => match decode_provenance_section(payload) {
                Err(_) => Vision10dBarrier::Deny {
                    reason: "provenance_decode_failed",
                },
                Ok(view) => match validate_provenance(&view) {
                    Ok(()) => Vision10dBarrier::Permit,
                    Err(_) => Vision10dBarrier::Deny {
                        reason: "provenance_invalid",
                    },
                },
            },
        },
    }
}

/// Convenience: true only when barrier is Permit.
pub fn vision_10d_may_load(bytes: &[u8], access: Vision10dAccess) -> bool {
    matches!(
        evaluate_vision_10d_barrier(bytes, access),
        Vision10dBarrier::Permit
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::container_10d::provenance_section::ProvenanceSidecar;
    use qualia_core_db::render::assets::Mesh;
    use qualia_core_db::render::compile_10d::{
        compile_mesh_to_10d_vision, compile_mesh_to_10d_vision_with_provenance,
    };
    use qualia_core_db::tensor::Tensor10D;

    fn tri_mesh() -> Mesh {
        Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 0.0],
        }
    }

    #[test]
    fn unattested_denied_for_citable() {
        let nodes = [Tensor10D::default()];
        let bytes = compile_mesh_to_10d_vision(&tri_mesh(), &nodes).unwrap();
        assert!(matches!(
            evaluate_vision_10d_barrier(&bytes, Vision10dAccess::CitableRequireProvenance),
            Vision10dBarrier::Deny {
                reason: "missing_provenance"
            }
        ));
        assert!(matches!(
            evaluate_vision_10d_barrier(&bytes, Vision10dAccess::BrowseAllowUnattested),
            Vision10dBarrier::Permit
        ));
    }

    #[test]
    fn attested_permits_citable() {
        let nodes = [Tensor10D::default()];
        let prov = ProvenanceSidecar::new(b"source-rgb-stub".to_vec(), "image/rgb8", "CC0");
        let bytes = compile_mesh_to_10d_vision_with_provenance(&tri_mesh(), &nodes, &prov).unwrap();
        assert!(matches!(
            evaluate_vision_10d_barrier(&bytes, Vision10dAccess::CitableRequireProvenance),
            Vision10dBarrier::Permit
        ));
    }
}
