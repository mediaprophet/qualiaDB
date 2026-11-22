//! Resolve the semantic pointers exposed by native M64 record codecs.

use super::{invalid, SemanticModelBuilder, RDF_GRAPH};
use crate::q42::m64_matter::{
    M64DecodedRecord, M64Header, M64PayloadInterval, M64ReadLimits, M64Reader, M64_RECORD_BYTES,
};
use std::{collections::BTreeSet, io};

const PROVENANCE: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
const ANCHORED_TO: &str = "https://qualiadb.org/ontology/hmc#anchoredTo";

impl SemanticModelBuilder {
    /// Read the M64 frame and provenance pointers without rewriting the source container.
    /// Zero is the native absent-provenance sentinel. Present pointers must resolve to a
    /// canonical Q42 spelling already admitted from the package's companion dictionaries.
    pub fn merge_m64(mut self, bytes: &[u8], resource_iri: &str) -> io::Result<Self> {
        let Some(raw) = bytes.get(..M64_RECORD_BYTES) else {
            return Err(invalid("M64 resource has no complete header"));
        };
        let header: M64Header = bytemuck::pod_read_unaligned(raw);
        header.validate().map_err(|error| invalid(error.to_string()))?;
        let chunks = usize::try_from(header.chunk_count)
            .map_err(|_| invalid("M64 chunk count exceeds address space"))?;
        let scratch_bytes = chunks.checked_mul(std::mem::size_of::<M64PayloadInterval>())
            .filter(|&size| size <= self.limit)
            .ok_or_else(|| invalid("M64 semantic inspection exceeds scratch budget"))?;
        if chunks > bytes.len() / M64_RECORD_BYTES || scratch_bytes > bytes.len() {
            return Err(invalid("M64 chunk directory exceeds the resource extent"));
        }
        let mut scratch = vec![M64PayloadInterval::default(); chunks];
        let reader = M64Reader::parse(bytes, M64ReadLimits {
            max_file_bytes: bytes.len(), max_chunks: chunks,
            max_records: (bytes.len() / M64_RECORD_BYTES) as u64,
        }, &mut scratch).map_err(|error| invalid(error.to_string()))?;
        if let Some(frame) = reader.lrm3() {
            let iri = self.resolve_iri(frame.anchor_did_hash)?.to_owned();
            self.fact(resource_iri, ANCHORED_TO, &iri, RDF_GRAPH)?;
        }
        let mut provenances = BTreeSet::new();
        for index in 0..chunks {
            let chunk = reader.chunk(index).ok_or_else(|| invalid("missing M64 chunk"))?;
            for index in 0..chunk.directory_entry().record_count as usize {
                let record = chunk.decode_record(index).map_err(|error| invalid(error.to_string()))?
                    .ok_or_else(|| invalid("missing M64 record"))?;
                let provenance = match record {
                    M64DecodedRecord::Point(row) => row.provenance_did_hash,
                    M64DecodedRecord::Holo(row) => row.provenance_did_hash,
                    M64DecodedRecord::PhaseJuncture(row) => row.provenance_did_hash,
                    M64DecodedRecord::Strata(_) | M64DecodedRecord::Shell(_) | M64DecodedRecord::BioTissue(_) => 0,
                };
                if provenance != 0 && !provenances.contains(&provenance) {
                    let iri = self.resolve_iri(provenance)?.to_owned();
                    self.fact(resource_iri, PROVENANCE, &iri, RDF_GRAPH)?;
                    provenances.insert(provenance);
                }
            }
        }
        Ok(self)
    }
}
