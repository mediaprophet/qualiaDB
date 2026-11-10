//! CPU decode for QualiaPortal `load_emf`.
//!
//! Sibling `Q42EMF64` (`.emf`) holds the field values. `.10d` MID2/FIEL names
//! the field. RGBA and AcousticPlane Hz are conversions of these cells.

use crate::q42::emf_container::{DenseEmfVolume, EmfCellRecord64, EmfContainerReader};
use crate::render::acoustic::sigma_to_center_frequency_hz;

/// Unpacked sibling `.emf` for GPU upload and AcousticPlane σ.
#[derive(Clone, Debug)]
pub struct LoadedEmf {
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

/// Parse `Q42EMF64` bytes and unpack Hilbert 8³ bricks into a dense xyz grid.
pub fn decode_emf_scientific(bytes: &[u8]) -> Result<LoadedEmf, String> {
    if bytes.len() < 8 || &bytes[..8] != b"Q42EMF64" {
        return Err("emf: expected Q42EMF64 magic".into());
    }
    let reader = EmfContainerReader::parse(bytes).map_err(|e| format!("emf: {e}"))?;
    let volume: DenseEmfVolume = reader
        .dense_volume()
        .map_err(|e| format!("emf volume: {e}"))?;
    if volume.cells.is_empty() {
        return Err("emf: empty dense volume".into());
    }
    Ok(LoadedEmf {
        nx: volume.nx,
        ny: volume.ny,
        nz: volume.nz,
        nt: volume.nt,
        cell_count: volume.cells.len() as u32,
        mean_sigma: volume.mean_sigma,
        frequency_hz: sigma_to_center_frequency_hz(volume.mean_sigma),
        bounds: volume.bounds_meters(),
        cells: volume.cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::emf_container::{
        emf_brick_voxel_linear_index, EmfCellRecord64, EmfContainerWriter, EMF_BRICK_CELLS,
    };
    use crate::q42::m64_matter::encode_hilbert3d_u64;

    fn sealed_green_brick() -> Vec<u8> {
        let mut writer = EmfContainerWriter::new([0.0, 0.0, 0.0], 0.5, [1, 1, 1], 0.0);
        let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        let idx = emf_brick_voxel_linear_index(0, 0, 0);
        cells[idx].e_field = [10.0, 0.0, 0.0];
        cells[idx].wavelength_nm = 550.0;
        cells[idx].confidence_alpha = 1.0;
        cells[idx].poynting_flux = cells[idx].poynting_flux_magnitude();
        writer
            .add_brick_chunk(encode_hilbert3d_u64(0, 0, 0, 2), &cells, 0, 1, 0)
            .expect("brick");
        writer.to_bytes().expect("serialize")
    }

    #[test]
    fn decode_rejects_non_emf_magic() {
        assert!(decode_emf_scientific(b"10d\0not-emf").is_err());
    }

    #[test]
    fn decode_unpacks_q42emf64_sigma_and_frequency() {
        let bytes = sealed_green_brick();
        let loaded = decode_emf_scientific(&bytes).expect("decode");
        assert_eq!((loaded.nx, loaded.ny, loaded.nz, loaded.nt), (8, 8, 8, 1));
        assert_eq!(loaded.cell_count, 512);
        assert!((loaded.mean_sigma - 0.5).abs() < 0.05);
        assert!(loaded.frequency_hz > 100.0 && loaded.frequency_hz < 2000.0);
        assert!((loaded.cells[0].e_magnitude() - 10.0).abs() < 1e-5);
        assert!((loaded.cells[0].effective_sigma() - 0.5).abs() < 1e-5);
    }

    /// Anatomy demo (and hosts) load field values from the sibling path:
    /// `foo.10d` → `foo.emf` (replace trailing extension only). Values live in
    /// Q42EMF64; they are never stored inside `.10d`.
    fn sibling_emf_url(mesh_url: &str) -> Option<String> {
        mesh_url
            .strip_suffix(".10d")
            .map(|stem| format!("{stem}.emf"))
    }

    #[test]
    fn sibling_emf_path_replaces_trailing_10d_extension_only() {
        assert_eq!(
            sibling_emf_url("working/heart/master/heart-exterior.10d").as_deref(),
            Some("working/heart/master/heart-exterior.emf")
        );
        assert_eq!(
            sibling_emf_url("distribution/members/authored/male/heart.10d").as_deref(),
            Some("distribution/members/authored/male/heart.emf")
        );
        assert_eq!(
            sibling_emf_url("archive/foo.10d-backup/mesh.10d").as_deref(),
            Some("archive/foo.10d-backup/mesh.emf")
        );
        assert_eq!(sibling_emf_url("mesh.emf"), None);
        assert_eq!(sibling_emf_url("mesh.10d.bak"), None);
    }
}
