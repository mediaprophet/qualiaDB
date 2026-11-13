//! One bounded accepted field, independent of GPU readiness and device lifetime.

use crate::q42::emf_container::{EmfCellRecord64, EmfExtensionMetadata, MAX_DENSE_CELLS};

/// Unpacked sibling field, retained independently of its device residency.
#[derive(Clone, Debug)]
pub struct LoadedEmf {
    pub container_version: u32,
    pub interpretation: Option<EmfExtensionMetadata>,
    pub nx: u32,
    pub ny: u32,
    pub nz: u32,
    pub nt: u32,
    pub cell_count: u32,
    pub mean_sigma: f32,
    pub frequency_hz: f32,
    pub bounds: [f32; 6],
    pub cells: Vec<EmfCellRecord64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmfBackend {
    Pending,
    WebGpu,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum EmfDisplayStatus {
    #[default]
    Empty = 0,
    Pending = 1,
    Uploaded = 2,
    Unavailable = 3,
}

impl EmfBackend {
    pub fn accepted_status(self) -> EmfDisplayStatus {
        match self {
            Self::Pending => EmfDisplayStatus::Pending,
            Self::WebGpu => EmfDisplayStatus::Uploaded,
            Self::Unavailable => EmfDisplayStatus::Unavailable,
        }
    }
}

impl EmfDisplayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Pending => "pending",
            Self::Uploaded => "uploaded",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Default)]
pub struct RetainedEmf {
    field: Option<LoadedEmf>,
    status: EmfDisplayStatus,
}

impl RetainedEmf {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn bounds(&self) -> Option<[f32; 6]> {
        self.field.as_ref().map(|field| field.bounds)
    }

    pub fn status(&self) -> EmfDisplayStatus {
        self.status
    }

    /// Decoder/metadata validation precedes this call. Check the dense shape
    /// again at the ownership boundary; no unbounded queue or duplicate cache.
    pub fn accept(
        &mut self,
        field: LoadedEmf,
        backend: EmfBackend,
        upload: impl FnOnce(&LoadedEmf) -> Result<(), String>,
    ) -> Result<(), String> {
        let count = [field.nx, field.ny, field.nz, field.nt]
            .into_iter()
            .try_fold(1usize, |n, d| n.checked_mul(d as usize));
        if !matches!(count, Some(n) if n > 0 && n <= MAX_DENSE_CELLS
            && n == field.cells.len() && n == field.cell_count as usize)
            || !field.bounds.iter().all(|v| v.is_finite())
            || !field.mean_sigma.is_finite()
        {
            return Err("emf: invalid or oversized retained dense field".into());
        }
        if backend == EmfBackend::WebGpu {
            // GPU upload itself admits all replacement resources before swapping.
            // An upload refusal leaves the previous accepted field/status intact.
            upload(&field)?;
        }
        self.field = Some(field);
        self.status = backend.accepted_status();
        Ok(())
    }

    /// Device loss invalidates residency, not accepted scientific data.
    pub fn invalidate(&mut self, pending: bool) {
        if self.field.is_some() {
            self.status = if pending {
                EmfDisplayStatus::Pending
            } else {
                EmfDisplayStatus::Unavailable
            };
        }
    }

    pub fn replay(
        &mut self,
        upload: impl FnOnce(&LoadedEmf) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(field) = &self.field {
            if let Err(error) = upload(field) {
                self.status = EmfDisplayStatus::Unavailable;
                return Err(error);
            }
            self.status = EmfDisplayStatus::Uploaded;
        }
        Ok(())
    }
}

#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub(crate) fn upload(
    gpu: &mut crate::render::gpu::PortalGpu,
    field: &LoadedEmf,
) -> Result<(), String> {
    let display: Vec<_> = field
        .cells
        .iter()
        .map(crate::render::gpu::emf_field_cell_from_scientific)
        .collect();
    gpu.emf_upload_field(
        &display,
        field.nx,
        field.ny,
        field.nz,
        field.nt,
        field.bounds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::emf_container::{
        EmfCellRecord64, EmfContainerReader, EmfContainerWriter, EMF_BRICK_CELLS,
    };
    use crate::q42::m64_matter::encode_hilbert3d_u64;

    fn field(amplitude: f32) -> LoadedEmf {
        let mut writer = EmfContainerWriter::new([0.0; 3], 0.5, [1; 3], 0.0);
        let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        cells[0].e_field = [amplitude, 0.0, 0.0];
        cells[0].wavelength_nm = 550.0;
        cells[0].confidence_alpha = 1.0;
        writer
            .add_brick_chunk(encode_hilbert3d_u64(0, 0, 0, 2), &cells, 0, 1, 0)
            .unwrap();
        let bytes = writer.to_bytes().unwrap();
        let reader = EmfContainerReader::parse(&bytes).unwrap();
        let volume = reader.dense_volume().unwrap();
        LoadedEmf {
            container_version: reader.header().version as u32,
            interpretation: reader
                .extension()
                .map(|extension| extension.metadata().clone()),
            nx: volume.nx,
            ny: volume.ny,
            nz: volume.nz,
            nt: volume.nt,
            cell_count: volume.cells.len() as u32,
            mean_sigma: volume.mean_sigma,
            // Ownership tests do not exercise acoustic conversion. That remains
            // covered by portal_load_emf's decoder tests and the browser bridge.
            frequency_hz: 440.0,
            bounds: volume.bounds_meters(),
            cells: volume.cells,
        }
    }

    #[test]
    fn early_load_replays_after_adoption_and_again_after_device_loss() {
        let mut state = RetainedEmf::default();
        state
            .accept(field(10.0), EmfBackend::Pending, |_| panic!("no device"))
            .unwrap();
        assert_eq!(state.status(), EmfDisplayStatus::Pending);
        for _ in 0..2 {
            state
                .replay(|retained| {
                    assert_eq!(retained.cells[0].e_field[0], 10.0);
                    assert_eq!(retained.cell_count, 512);
                    Ok(())
                })
                .unwrap();
            assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
            state.invalidate(true);
            assert_eq!(state.status(), EmfDisplayStatus::Pending);
        }
    }

    #[test]
    fn failed_replacement_preserves_accepted_field_and_residency() {
        let mut state = RetainedEmf::default();
        state
            .accept(field(10.0), EmfBackend::WebGpu, |_| Ok(()))
            .unwrap();
        assert!(state
            .accept(field(20.0), EmfBackend::WebGpu, |_| Err("budget".into()))
            .is_err());
        assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
        let mut invalid = field(30.0);
        invalid.nx = u32::MAX;
        assert!(state
            .accept(invalid, EmfBackend::WebGpu, |_| panic!(
                "invalid field uploaded"
            ))
            .is_err());
        state
            .replay(|retained| {
                assert_eq!(retained.cells[0].e_field[0], 10.0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn unavailable_backend_retains_data_and_failed_replay_can_retry() {
        let mut state = RetainedEmf::default();
        state
            .accept(field(20.0), EmfBackend::Unavailable, |_| {
                panic!("WebGL2 cannot upload EMF")
            })
            .unwrap();
        assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
        assert!(state.replay(|_| Err("device limit".into())).is_err());
        assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
        state
            .replay(|retained| {
                assert_eq!(retained.cells[0].e_field[0], 20.0);
                Ok(())
            })
            .unwrap();
        assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
        state.invalidate(false);
        assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
    }

    #[test]
    fn pending_field_becomes_unavailable_after_initialization_refusal_without_fallback() {
        let mut state = RetainedEmf::default();
        state
            .accept(field(30.0), EmfBackend::Pending, |_| {
                panic!("not initialized")
            })
            .unwrap();
        state.invalidate(false);
        assert_eq!(state.status(), EmfDisplayStatus::Unavailable);
        state
            .replay(|retained| {
                assert_eq!(retained.cells[0].e_field[0], 30.0);
                Ok(())
            })
            .unwrap();
        assert_eq!(state.status(), EmfDisplayStatus::Uploaded);
    }
}
