//! Bounded cold-path import of a companion volume into the package graph.

use super::{invalid, LexiconEntry, SemanticModelBuilder};
use crate::{
    q42_lex::Q42LexMmap,
    q42_volume::{
        payload_catalogue::{decode_payload_catalogue_with_bodies, sha256_hex_bytes},
        OpfsSliceRangeSource, Q42RangeQueryCursor, Q42RangeQueryPattern, Q42RangeQueryPlan,
        Q42RangeVolume, FLAG_SANCTUARY, FLAG_VOLUME_ROOT, MAX_COMPRESSED_SUPERBLOCK_SIZE,
        SUPERBLOCK_SIZE,
    },
    NQuin,
};
use std::{collections::{btree_map, hash_map}, io};

fn slice(bytes: &[u8], offset: u64, length: u64) -> io::Result<&[u8]> {
    let start = usize::try_from(offset).map_err(|_| invalid("Q42 offset exceeds address space"))?;
    let end = offset.checked_add(length).and_then(|end| usize::try_from(end).ok())
        .ok_or_else(|| invalid("Q42 extent exceeds address space"))?;
    bytes.get(start..end).ok_or_else(|| invalid("Q42 extent is outside the source volume"))
}

impl SemanticModelBuilder {
    /// A restricted journal projection must keep its access classification.
    pub fn restrict_to_sanctuary(&mut self) {
        self.sanctuary = true;
    }

    /// Return the imported snapshot rows for an authoring transaction's baseline.
    pub fn snapshot_quins(&self) -> &[NQuin] {
        &self.quins
    }

    /// Bind a canonical IRI for an authored transaction. Existing terms can only
    /// be reused with the same spelling.
    pub fn bind_iri(&mut self, iri: &str) -> io::Result<u64> {
        if !super::valid_iri(iri) {
            return Err(invalid("transaction lexical binding requires an absolute IRI"));
        }
        let hash = crate::q_hash(iri);
        self.register(hash, iri)?;
        Ok(hash)
    }

    /// Check all resource tokens on a newly authored row against its dictionary.
    pub fn check_quin_tokens(&self, row: &NQuin) -> io::Result<()> {
        let required = [row.subject, row.predicate, row.object];
        if required.iter().any(|hash| *hash == 0 || !self.lex.contains_key(hash))
            || (row.context != 0 && !self.lex.contains_key(&row.context))
        {
            return Err(invalid("journal row contains an unbound Q42 token"));
        }
        Ok(())
    }

    /// Export the committed projection with its original lexicon and PCAT content.
    /// A graph from which all statements were removed seals an empty Q42 volume.
    pub fn seal_projection(mut self, rows: impl IntoIterator<Item = NQuin>) -> io::Result<Vec<u8>> {
        self.source_bytes -= self.quins.len() * std::mem::size_of::<NQuin>();
        self.quins.clear();
        for row in rows {
            self.push(row)?;
        }
        self.seal_volume()
    }

    /// Merge a self-contained Q42's tokens, facts, and body-bearing PCAT payloads.
    /// The original companion remains intact in the HMC. A volume root or unresolved
    /// catalogue body requires additional source data and must be resolved before packing.
    pub fn merge_q42(mut self, bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > self.limit {
            return Err(invalid("companion Q42 exceeds semantic model byte budget"));
        }
        if bytes.starts_with(b"Q42LOGIC") {
            return self.merge_q42_logic(bytes);
        }
        let volume = Q42RangeVolume::open(OpfsSliceRangeSource::new(bytes))?;
        let header = volume.header();
        if header.flags & FLAG_VOLUME_ROOT != 0 {
            return Err(invalid("companion Q42 must be a self-contained volume"));
        }
        self.sanctuary |= header.flags & FLAG_SANCTUARY != 0;
        let dictionary = Q42LexMmap::from_bytes(slice(bytes, header.lex_offset, header.lex_length)?)
            .map_err(|error| invalid(error.to_string()))?;
        for index in 0..dictionary.entry_count() {
            let hash = dictionary.hash_at(index).ok_or_else(|| invalid("invalid Q42LEX index"))?;
            if let Some((prefix, suffix)) = dictionary.string_parts_at(index) {
                if suffix.is_empty() {
                    self.register_spelling(hash, prefix)?;
                } else {
                    self.register_spelling(hash, &format!("{prefix}{suffix}"))?;
                }
                continue;
            }
            let entry = if let Some(triple) = dictionary.lookup_embedded_triple(hash) {
                LexiconEntry::EmbeddedTriple(triple)
            } else if let Some(webid) = dictionary.lookup_webizen_identity(hash) {
                LexiconEntry::Webizen(webid.to_owned())
            } else {
                return Err(invalid("unsupported Q42LEX token"));
            };
            if hash == 0 {
                return Err(invalid("Q42LEX token uses the zero hash"));
            }
            match self.lex.entry(hash) {
                hash_map::Entry::Occupied(existing) => {
                    let same = match (existing.get(), &entry) {
                        (LexiconEntry::EmbeddedTriple(a), LexiconEntry::EmbeddedTriple(b)) => a == b,
                        (LexiconEntry::Webizen(a), LexiconEntry::Webizen(b)) => a == b,
                        _ => false,
                    };
                    if !same {
                        return Err(invalid(format!("semantic hash collision at {hash:#018x}")));
                    }
                }
                hash_map::Entry::Vacant(slot) => {
                    let bytes = match &entry {
                        LexiconEntry::Webizen(webid) => webid.len().saturating_add(16),
                        _ => 40,
                    };
                    Self::reserve(&mut self.source_bytes, self.limit, bytes)?;
                    slot.insert(entry);
                }
            }
        }
        if let Some((offset, length)) = header.payload_catalogue_range() {
            let payloads = decode_payload_catalogue_with_bodies(slice(bytes, offset, length)?)
                .map_err(invalid)?;
            for payload in payloads {
                if payload.utf8_body.len() as u64 != payload.entry.byte_length
                    || !sha256_hex_bytes(payload.utf8_body.as_bytes())
                        .eq_ignore_ascii_case(&payload.entry.sha256_hex)
                {
                    return Err(invalid(format!(
                        "Q42 catalogue body is unavailable for {}", payload.entry.payload_key,
                    )));
                }
                match self.payloads.entry(payload.entry.payload_key.clone()) {
                    btree_map::Entry::Occupied(old) if old.get() == &payload => {}
                    btree_map::Entry::Occupied(_) => {
                        return Err(invalid(format!(
                            "conflicting Q42 catalogue payload {}", payload.entry.payload_key,
                        )));
                    }
                    btree_map::Entry::Vacant(slot) => {
                        let bytes = payload.utf8_body.len()
                            .saturating_add(payload.entry.payload_key.len())
                            .saturating_add(payload.entry.mime_type.len())
                            .saturating_add(payload.entry.role.len())
                            .saturating_add(payload.entry.language.as_deref().map_or(0, str::len))
                            .saturating_add(128);
                        Self::reserve(&mut self.source_bytes, self.limit, bytes)?;
                        slot.insert(payload);
                    }
                }
            }
        }
        if volume.block_count() == 0 {
            return Ok(self);
        }
        let mut compressed = vec![0; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = vec![0; SUPERBLOCK_SIZE];
        let mut out = vec![<NQuin as bytemuck::Zeroable>::zeroed(); crate::QUINS_PER_BLOCK];
        let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern::default());
        let mut cursor = Q42RangeQueryCursor::default();
        loop {
            let page = volume.execute_query_page_into(
                plan, cursor, &mut compressed, &mut decoded, &mut out,
            )?;
            for &row in &out[..page.returned] {
                self.push(row)?;
            }
            let Some(next) = page.next_cursor else { break };
            cursor = next;
        }
        Ok(self)
    }

    fn merge_q42_logic(mut self, bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 64 || &bytes[0..8] != b"Q42LOGIC" {
            return Err(invalid("invalid Q42LOGIC header"));
        }
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let quin_count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let expected_len = 64usize
            .checked_add(quin_count.checked_mul(48).ok_or_else(|| invalid("Q42LOGIC size overflow"))?)
            .ok_or_else(|| invalid("Q42LOGIC size overflow"))?;
        if version != 1 || bytes.len() != expected_len {
            return Err(invalid("invalid Q42LOGIC version or record extent"));
        }
        for raw in bytes[64..].chunks_exact(48) {
            let subject = u64::from_le_bytes(raw[0..8].try_into().unwrap());
            let predicate = u64::from_le_bytes(raw[8..16].try_into().unwrap());
            let object = u64::from_le_bytes(raw[16..24].try_into().unwrap());
            let context = u64::from_le_bytes(raw[24..32].try_into().unwrap());
            let metadata = u64::from_le_bytes(raw[32..40].try_into().unwrap());
            let wire_parity = u64::from_le_bytes(raw[40..48].try_into().unwrap());
            if wire_parity != (subject ^ predicate ^ object ^ context)
                && wire_parity != NQuin::calculate_parity(subject, predicate, object, context, metadata)
            {
                return Err(invalid("Q42LOGIC record failed parity check"));
            }
            for (role, hash) in [("s", subject), ("p", predicate), ("o", object), ("c", context)] {
                if hash != 0 && !self.lex.contains_key(&hash) {
                    self.register_spelling(hash, &format!("urn:qualia:q42logic:{role}:{hash:016x}"))?;
                }
            }
            self.push(NQuin {
                subject,
                predicate,
                object,
                context,
                metadata,
                parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
            })?;
        }
        Ok(self)
    }
}
