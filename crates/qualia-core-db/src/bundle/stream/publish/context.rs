use super::PlainPublisher;
use crate::bundle::stream::descriptor::*;
impl PlainPublisher<'_, '_> {
    pub fn emit_context_page(
        &mut self,
        page: usize,
        out: &mut [u8],
        framing: &mut [u8],
        mut sink: impl FnMut(&[u8], PageRef) -> Result<(), DescriptorError>,
    ) -> Result<usize, DescriptorError> {
        self.live()?;
        if self.context_id.is_some() || page != self.context_next || page >= self.page_count() {
            return Err(DescriptorError::Page);
        }
        let first = page * PAGE_ROWS;
        let count = (self.entry_count() - first).min(PAGE_ROWS);
        let mut rows = [ContextEntry::EMPTY; PAGE_ROWS];
        let mut seen = 0;
        for (i, row) in self.index.entries().skip(first).take(count).enumerate() {
            let actual = row.map_err(|_| DescriptorError::Source)?;
            rows[i] = ContextEntry {
                id: self.assignments[first + i].id,
                length: actual.length(),
            };
            seen += 1;
        }
        if seen != count {
            return Err(DescriptorError::Coverage);
        }
        let n = encode_context_page_into(page as u32, first as u32, &rows[..count], out)?;
        let digest = domain_digest(DigestDomain::ContextPage, &out[..n], framing)?;
        let used = self.charge(n)?;
        let reference = PageRef {
            number: page as u32,
            first: first as u32,
            rows: count as u32,
            encoded_length: n as u32,
            digest,
        };
        self.poisoned = true;
        sink(&out[..n], reference).map_err(|_| DescriptorError::Sink)?;
        self.context_refs[page] = reference;
        self.context_next += 1;
        self.used = used;
        self.poisoned = false;
        Ok(n)
    }
    /// Idempotent encoding of the fixed completed context; callbacks are not repeated.
    pub fn encode_context_root(
        &mut self,
        out: &mut [u8],
        framing: &mut [u8],
    ) -> Result<usize, DescriptorError> {
        self.live()?;
        if self.context_next != self.page_count() {
            return Err(DescriptorError::Incomplete);
        }
        let input = ContextInput {
            parameters: self.parameters,
            source: self.source_commit,
            entries: self.entry_count(),
            pages: &self.context_refs[..self.page_count()],
        };
        let n = encode_context_into(&input, out)?;
        let digest = domain_digest(DigestDomain::Context, &out[..n], framing)?;
        if let Some(want) = self.context_id {
            if digest != want {
                return Err(DescriptorError::Commitment);
            }
        } else {
            self.used = self.charge(n)?;
            self.context_id = Some(digest);
        }
        Ok(n)
    }
}
