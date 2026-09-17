//! Load one Q42 volume (or volume-set child) and sieve a field. GPU when
//! eligible; CPU floor always.

use crate::query::graph_accel::sieve::{sieve_eq, SieveOutcome};
use crate::query::graph_accel::QuinField;
use crate::NQuin;

/// Read all Quins from `path` (single-file volume) into `scratch`, then compact
/// matches into `out`. Fails closed if the volume is larger than `scratch`.
pub fn sieve_volume_file(
    path: &std::path::Path,
    field: QuinField,
    needle: u64,
    scratch: &mut [NQuin],
    out: &mut [NQuin],
) -> Result<(SieveOutcome, usize), String> {
    let vol = crate::q42_volume::Q42Volume::open(path).map_err(|e| e.to_string())?;
    let loaded = vol.read_all_quins().map_err(|e| e.to_string())?;
    if loaded.len() > scratch.len() {
        return Err(format!(
            "volume has more Quins than the caller scratch ({} > {})",
            loaded.len(),
            scratch.len()
        ));
    }
    let n = loaded.len();
    scratch[..n].copy_from_slice(&loaded);
    let outcome = sieve_eq(&scratch[..n], field, needle, out);
    Ok((outcome, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::ingest::{streaming_import_rdf_with_mode, IngestMode};

    #[test]
    fn sieves_ingested_ntriples() {
        let dir = tempfile::TempDir::new().unwrap();
        let input = dir.path().join("in.nt");
        let output = dir.path().join("out.q42");
        std::fs::write(
            &input,
            "<http://ex/s1> <http://ex/p> <http://ex/keep> .\n\
             <http://ex/s2> <http://ex/p> <http://ex/drop> .\n\
             <http://ex/s3> <http://ex/p> <http://ex/keep> .\n",
        )
        .unwrap();
        streaming_import_rdf_with_mode(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            IngestMode::Complete,
        )
        .unwrap();
        let needle = crate::query::ingest_formats::object_iri_hash("http://ex/keep");
        let mut scratch = vec![NQuin::default(); 16];
        let mut out = vec![NQuin::default(); 16];
        let (hit, scanned) =
            sieve_volume_file(&output, QuinField::Object, needle, &mut scratch, &mut out).unwrap();
        assert_eq!(scanned, 3);
        assert_eq!(hit.written, 2);
    }
}
