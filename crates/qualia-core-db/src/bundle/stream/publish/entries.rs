use super::PlainPublisher;
use crate::bundle::stream::descriptor::*;
use crate::bundle::stream::proof::{
    OutboardNode, ProofError, Representation, TreeBuilder, TreeSpec,
};
impl PlainPublisher<'_, '_> {
    pub fn emit_entry_page(
        &mut self,
        page: usize,
        out: &mut [u8],
        framing: &mut [u8],
        mut observe: impl FnMut(u32, OutboardNode) -> Result<(), DescriptorError>,
        mut sink: impl FnMut(&[u8], PageRef) -> Result<(), DescriptorError>,
    ) -> Result<usize, DescriptorError> {
        self.live()?;
        let context = self.context_id.ok_or(DescriptorError::Incomplete)?;
        if page != self.entry_next || page >= self.page_count() {
            return Err(DescriptorError::Page);
        }
        let first = page * PAGE_ROWS;
        let count = (self.entry_count() - first).min(PAGE_ROWS);
        let mut rows = [EntryRecord::EMPTY; PAGE_ROWS];
        let mut seen = 0;
        // Poison BEFORE the first observer so an error or unwind can never mint a final candidate.
        self.poisoned = true;
        for (i, entry) in self.index.entries().skip(first).take(count).enumerate() {
            let entry = entry.map_err(|_| DescriptorError::Source)?;
            let assignment = self.assignments[first + i];
            let spec = TreeSpec::new(
                context,
                assignment.id,
                self.parameters.generation,
                Representation::Plaintext,
                0,
                entry.length(),
            )
            .map_err(|_| DescriptorError::Proof)?;
            let start = usize::try_from(entry.offset()).map_err(|_| DescriptorError::Overflow)?;
            let end = entry
                .offset()
                .checked_add(entry.length())
                .and_then(|n| usize::try_from(n).ok())
                .ok_or(DescriptorError::Overflow)?;
            let bytes = self.source.get(start..end).ok_or(DescriptorError::Source)?;
            let mut tree = TreeBuilder::new(spec);
            for chunk in bytes.chunks(LEAF_BYTES as usize) {
                tree.append(chunk, |node| {
                    observe(assignment.id, node).map_err(|_| ProofError::Sink)
                })
                .map_err(|e| {
                    if matches!(e, ProofError::Sink | ProofError::Poisoned) {
                        DescriptorError::Sink
                    } else {
                        DescriptorError::Proof
                    }
                })?;
            }
            let root = tree
                .finish(|node| observe(assignment.id, node).map_err(|_| ProofError::Sink))
                .map_err(|e| {
                    if matches!(e, ProofError::Sink | ProofError::Poisoned) {
                        DescriptorError::Sink
                    } else {
                        DescriptorError::Proof
                    }
                })?;
            rows[i] = EntryRecord {
                id: assignment.id,
                ordinal: (first + i) as u32,
                kind: entry.kind(),
                inner_version: assignment.inner_version,
                offset: entry.offset(),
                length: entry.length(),
                root,
            };
            seen += 1;
        }
        if seen != count {
            return Err(DescriptorError::Coverage);
        }
        let n = encode_entry_page_into(context, page as u32, first as u32, &rows[..count], out)?;
        let digest = domain_digest(DigestDomain::EntryPage, &out[..n], framing)?;
        let used = self.charge(n)?;
        let reference = PageRef {
            number: page as u32,
            first: first as u32,
            rows: count as u32,
            encoded_length: n as u32,
            digest,
        };
        sink(&out[..n], reference).map_err(|_| DescriptorError::Sink)?;
        self.entry_refs[page] = reference;
        self.entry_next += 1;
        self.used = used;
        self.poisoned = false;
        Ok(n)
    }
}
