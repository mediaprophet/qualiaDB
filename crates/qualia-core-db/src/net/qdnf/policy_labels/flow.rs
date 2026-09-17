//! Runtime-owned monotonically restrictive job label context.

use super::join::join_one;
use super::types::{LabelFields, VerifiedLabel};
use crate::net::qdnf::errors::QdnfError;

pub struct JobLabelContext {
    label: LabelFields,
    sealed: bool,
}

impl JobLabelContext {
    pub fn new(request: LabelFields) -> Result<Self, QdnfError> {
        if request.confidentiality.is_unknown() {
            return Err(QdnfError::Conflict);
        }
        Ok(Self {
            label: request,
            sealed: false,
        })
    }

    pub fn join_dependency(&mut self, dep: &VerifiedLabel) -> Result<(), QdnfError> {
        if self.sealed {
            return Err(QdnfError::StaleGeneration);
        }
        join_one(&mut self.label, dep.fields())
    }

    pub fn seal(&mut self) {
        self.sealed = true;
    }

    #[inline]
    pub const fn is_sealed(&self) -> bool {
        self.sealed
    }

    #[inline]
    pub const fn current(&self) -> &LabelFields {
        &self.label
    }
}

/// Join the object label before any protected bytes are copied.
/// A failed join copies 0 bytes and leaves `dst` untouched.
pub fn protected_read(
    ctx: &mut JobLabelContext,
    object_label: &VerifiedLabel,
    src: &[u8],
    dst: &mut [u8],
) -> Result<usize, QdnfError> {
    if let Err(err) = ctx.join_dependency(object_label) {
        return Err(err);
    }
    if src.len() > dst.len() {
        return Err(QdnfError::Capacity);
    }
    dst[..src.len()].copy_from_slice(src);
    Ok(src.len())
}
