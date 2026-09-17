//! Conservative sensitivity projection. Missing labels fail closed.

use super::flow::JobLabelContext;
use super::types::{Confidentiality, LabelFields};
use crate::net::qdnf::errors::QdnfError;

/// Derived output sinks that must inherit the job context. None may lower handling.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivedSink {
    Reply = 1,
    Summary = 2,
    Translation = 3,
    Embedding = 4,
    ModelContext = 5,
    QueryResult = 6,
    Export = 7,
    Log = 8,
    Receipt = 9,
    Backup = 10,
}

/// Core sensitivity cache is a projection, not the full label.
/// Unknown has no rank and never becomes Public.
pub fn project_sensitivity(fields: &LabelFields) -> Result<u8, QdnfError> {
    fields.confidentiality.lattice_rank()
}

pub fn require_labelled(fields: &LabelFields) -> Result<(), QdnfError> {
    if fields.confidentiality == Confidentiality::Unknown {
        return Err(QdnfError::Denied);
    }
    if fields.issuer.is_zero() {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

/// Protected content with a missing or unknown label fails closed.
pub fn require_labelled_content(
    protected_content: bool,
    fields: Option<&LabelFields>,
) -> Result<Confidentiality, QdnfError> {
    match (protected_content, fields) {
        (true, None) => Err(QdnfError::Incomplete),
        (false, None) => Ok(Confidentiality::C0Public),
        (_, Some(fields)) => {
            if fields.confidentiality.is_unknown() {
                return Err(QdnfError::Denied);
            }
            Ok(fields.confidentiality)
        }
    }
}

/// Inherit the current job label for any derived sink. Unknown fails closed.
pub fn project_sink(ctx: &JobLabelContext, _sink: DerivedSink) -> Result<LabelFields, QdnfError> {
    let current = *ctx.current();
    if current.confidentiality.is_unknown() {
        return Err(QdnfError::Incomplete);
    }
    Ok(current)
}
