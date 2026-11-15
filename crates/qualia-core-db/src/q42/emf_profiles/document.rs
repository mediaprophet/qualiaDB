//! Shared metadata is admitted once; selected payloads remain independently verified.
use super::*;

/// A nonforgeable shared admission for the current EMF registry. Opening does not
/// validate every payload; each chunk call verifies and admits that selected chunk.
/// The caller may retain immutable returned chunks for hot access without rehashing.
pub struct ProfileReader<'v, 'a> {
    view: &'v StructuralView<'a>,
    admission: semantics::SharedAdmission<'v, 'a>,
    limits: Limits,
    payload_work: u64,
}
impl<'v, 'a> ProfileReader<'v, 'a> {
    pub fn open(
        view: &'v StructuralView<'a>,
        scratch: &mut Scratch<'_>,
        limits: Limits,
        total_payload_work: u64,
    ) -> Result<Self, Error> {
        if view.family() != super::super::field_envelope::Family::EmfV4Candidate {
            return Err(Error::UnsupportedProfile);
        }
        let admission = semantics::admit_shared(view, &ProfileRegistry, scratch, limits.shared)
            .map_err(shared_error)?;
        Ok(Self {
            view,
            admission,
            limits,
            payload_work: total_payload_work,
        })
    }
    pub fn entry_count(&self) -> u32 {
        self.view.entry_count()
    }
    pub fn remaining_payload_work(&self) -> u64 {
        self.payload_work
    }
    pub fn remaining_shared_work(&self) -> u64 {
        self.admission.context.remaining_work()
    }
    /// Charge a conservative aggregate budget before hashing/scanning any payload.
    /// Three units per stored byte cover selected digest/record scan plus a validity
    /// mask (one byte per record, no larger than the selected raw payload). Per-chunk
    /// limits and shared graph/reference work limits also remain enforced. Failed
    /// calls consume their admitted budget rather than permitting unlimited retries.
    pub fn chunk(&mut self, entry_index: u32) -> Result<ProfileChunk<'a>, Error> {
        let entry = self.view.entry(entry_index).map_err(Error::Storage)?;
        let cost = entry.stored_bytes().checked_mul(3).ok_or(Error::Overflow)?;
        self.payload_work = self.payload_work.checked_sub(cost).ok_or(Error::Limit)?;
        super::chunk::admit_with_context(
            self.view,
            &self.admission.context,
            entry_index,
            self.limits,
        )
    }
}
