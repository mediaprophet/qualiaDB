//! Current EMF typed admission and bounded local-frame display projection.
//! Exact source, physical vectors/units, clocks and frames remain scientific data;
//! display magnitude does not imply carrier, colour, sigma or acoustic frequency.
use crate::q42::{
    emf_profiles as profiles,
    field_envelope::{self as env, Family, StructuralView},
    field_semantics as sem,
};
#[path = "portal_load_emf/admission.rs"]
mod admission;
#[path = "portal_load_emf/project.rs"]
mod project;
use crate::render::portal_emf_state::MAX_DISPLAY_CELLS;
pub use crate::render::portal_emf_state::{DisplayEmfCell, DisplaySelection, LoadedEmf};
#[derive(Clone, Copy, Debug)]
pub struct PortalEmfLimits {
    pub storage: env::Limits,
    pub profiles: profiles::Limits,
    pub total_payload_work: u64,
    pub projection_work: u64,
    pub source_bytes: usize,
    pub peak_owned_bytes: usize,
    pub display_cells: usize,
    pub time_slices: usize,
}
impl Default for PortalEmfLimits {
    fn default() -> Self {
        Self {
            storage: env::Limits::default(),
            profiles: profiles::Limits::default(),
            total_payload_work: 512 << 20,
            projection_work: 64 << 20,
            source_bytes: 128 << 20,
            peak_owned_bytes: 256 << 20,
            display_cells: MAX_DISPLAY_CELLS,
            time_slices: 4096,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Storage(env::Error),
    Profile(profiles::Error),
    Limit,
    Allocation,
    Overflow,
    UnsupportedProjection(&'static str),
    AmbiguousSelection,
    SelectionMissing,
    Overlap,
    OrphanValidityMask,
}
impl From<env::Error> for Error {
    fn from(e: env::Error) -> Self {
        Self::Storage(e)
    }
}
impl From<profiles::Error> for Error {
    fn from(e: profiles::Error) -> Self {
        Self::Profile(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "emf: ")?;
        match self {
            Self::UnsupportedProjection(why) => f.write_str(why),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for Error {}
pub(super) fn charge(work: &mut u64, n: u64) -> Result<(), Error> {
    *work = work.checked_sub(n).ok_or(Error::Limit)?;
    Ok(())
}
pub(super) fn vector<T>(len: usize, budget: usize) -> Result<Vec<T>, Error> {
    let bytes = len
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(Error::Overflow)?;
    if bytes > budget {
        return Err(Error::Limit);
    }
    let mut v = Vec::new();
    v.try_reserve_exact(len).map_err(|_| Error::Allocation)?;
    Ok(v)
}
/// Default display selects only an unambiguous mode0 grid, with every exact tick.
/// Independent metadata inspection remains available for unsupported projections.
pub fn decode_emf_scientific(bytes: &[u8]) -> Result<LoadedEmf, String> {
    decode_emf_with_selection(bytes, None, PortalEmfLimits::default()).map_err(|e| e.to_string())
}
pub fn decode_emf_with_selection(
    bytes: &[u8],
    selection: Option<DisplaySelection>,
    limits: PortalEmfLimits,
) -> Result<LoadedEmf, Error> {
    if bytes.len() > limits.source_bytes {
        return Err(Error::Limit);
    }
    let view = StructuralView::open(bytes, Family::EmfV4Candidate, limits.storage)?;
    let mut work = limits.projection_work;
    let chunks = admission::admit_all(&view, limits, &mut work)?;
    project::project(bytes, &view, &chunks, selection, limits, &mut work)
}

#[cfg(test)]
#[path = "portal_load_emf/tests.rs"]
mod tests;
