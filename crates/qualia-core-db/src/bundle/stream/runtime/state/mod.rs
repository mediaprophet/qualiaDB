//! Persistent exact runtime state with short canonical parent scopes.
//! State moves through the EXISTING transition implementation and restores on
//! every return/error/unwind. No new ledger or duplicate active tokens.
mod access;
mod close;
mod data;
mod loans;
mod parents;
mod preflight;
mod restore;
mod scope;
use crate::net::peer::runtime::incarnation::CanonicalIncarnation;
use crate::net::qdnf::errors::QdnfError as E;
pub(crate) use access::RuntimeAccess;
pub(crate) use close::RuntimeCloseError;
pub(super) use data::RuntimeClosePhase;
use data::RuntimeData;
pub(crate) use parents::RuntimeParents;
#[must_use = "drain real I/O and consume try_close; lost state retains canonical charges"]
pub(crate) struct RetainedRuntime<'parent, 'host> {
    parents: &'parent RuntimeParents<'host>,
    registration: CanonicalIncarnation,
    data: Option<RuntimeData>,
}
/// Actual retained shell + temporary legacy wrapper + guard/access footprint.
/// Scope moves at most2*size_of<RuntimeData> logical bytes; this is NOT zero-copy.
fn charged_bytes() -> Result<u64, E> {
    let bytes = core::mem::size_of::<RetainedRuntime<'static, 'static>>()
        .checked_add(core::mem::size_of::<RuntimeData>())
        .and_then(|n| n.checked_add(core::mem::size_of::<restore::Restore<'static, 'static>>()))
        .and_then(|n| n.checked_add(core::mem::size_of::<loans::ParentLoans<'static>>()))
        .and_then(|n| n.checked_add(core::mem::size_of::<RuntimeParents<'static>>()))
        .and_then(|n| n.checked_add(core::mem::size_of::<RuntimeAccess<'static, 'static>>()))
        .ok_or(E::Range)?;
    u64::try_from(bytes).map_err(|_| E::Range)
}
#[cfg(test)]
mod tests;
