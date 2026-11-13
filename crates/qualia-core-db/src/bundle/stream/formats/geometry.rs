//! Geometry-only closure; optional material, rig, topology and field sidecars
//! are not admitted by this view and must be validated before those uses.
use super::{FormatError as E, TenDSectionView};
use crate::container_10d::{mesh_section::MeshSectionView, section::SectionType};
impl<'a> TenDSectionView<'a> {
    /// Complete vertex/index geometry from the original authenticated section.
    /// This is neither a whole-container receipt nor a current disclosure grant.
    pub fn geometry(&self) -> Result<MeshSectionView<'a>, E> {
        if self.descriptor().typ() != Some(SectionType::QuantizedMesh) {
            return Err(E::Unsupported);
        }
        MeshSectionView::parse(self.bytes()).map_err(|_| E::Integrity)
    }
}
