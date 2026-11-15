use super::PlainPublisher;
use crate::bundle::stream::descriptor::*;
impl PlainPublisher<'_, '_> {
    /// Re-encoding is idempotent and emits no sink callbacks. Output remains a disclosure candidate.
    pub fn finish(
        &mut self,
        context_root: &[u8],
        out: &mut [u8],
        framing: &mut [u8],
    ) -> Result<CandidateIdentity, DescriptorError> {
        self.live()?;
        if self.context_next != self.page_count() || self.entry_next != self.page_count() {
            return Err(DescriptorError::Incomplete);
        }
        let context = self.context_id.ok_or(DescriptorError::Incomplete)?;
        if domain_digest(DigestDomain::Context, context_root, framing)? != context {
            return Err(DescriptorError::Context);
        }
        let parsed = decode_context(context_root)?;
        if parsed.parameters() != self.parameters
            || parsed.source() != self.source_commit
            || parsed.entry_count() != self.entry_count()
        {
            return Err(DescriptorError::Context);
        }
        parsed
            .page_refs()
            .matches(&self.context_refs[..self.page_count()])?;
        let input = BodyInput {
            context_id: context,
            context_root,
            generation: self.parameters.generation,
            predecessor: self.parameters.predecessor,
            entries: self.entry_count(),
            pages: &self.entry_refs[..self.page_count()],
        };
        let n = encode_body_into(&input, out)?;
        let descriptor = domain_digest(DigestDomain::Body, &out[..n], framing)?;
        let identity = CandidateIdentity {
            descriptor,
            context,
            generation: self.parameters.generation,
            entries: self.entry_count() as u32,
        };
        if let Some(want) = self.completed {
            if want != identity {
                return Err(DescriptorError::Commitment);
            }
        } else {
            self.used = self.charge(n)?;
            self.completed = Some(identity);
        }
        Ok(identity)
    }
    /// Canonical body length without a second encoding; useful after a successful finish.
    pub fn body_encoded_length(&self, context_root: &[u8]) -> Result<usize, DescriptorError> {
        self.live()?;
        let context = self.context_id.ok_or(DescriptorError::Incomplete)?;
        let input = BodyInput {
            context_id: context,
            context_root,
            generation: self.parameters.generation,
            predecessor: self.parameters.predecessor,
            entries: self.entry_count(),
            pages: &self.entry_refs[..self.page_count()],
        };
        body_encoded_len(&input)
    }
}
