//! Standalone `.emf` (`EMF-64`) Electromagnetic & Wave-Optical Field Container Library.
//!
//! Output is strictly cache-aligned (64-byte headers, directory rows, and cell records)
//! and little-endian, featuring 3D Hilbert volumetric brick clustering, CRC-32C,
//! and complete 256-bit BLAKE3 digests. Legacy prefix checks are migration-only.

pub mod chunk_directory;
pub mod extension;
pub mod header;
pub mod reader;
pub mod writer;

pub use chunk_directory::*;
pub use header::*;
pub use extension::*;
pub use reader::*;
pub use writer::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::m64_matter::brick3d::encode_hilbert3d_u64;

    #[test]
    fn test_emf_container_roundtrip_and_brick_lookup() {
        let mut writer = EmfContainerWriter::new(
            [10.0, 20.0, 30.0],
            0.5,
            [2, 2, 2],
            1_700_000_000.0,
        );

        let order = 2; // 4x4x4 brick grid
        // Create 2 bricks with distinct cells
        let mut brick0_cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        let mut brick1_cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];

        // Put a marker cell at local (3, 2, 1) in brick (0, 0, 0)
        let local_idx0 = emf_brick_voxel_linear_index(3, 2, 1);
        brick0_cells[local_idx0].e_field = [100.0, 200.0, 300.0];
        brick0_cells[local_idx0].b_field = [0.003, -0.001, 0.002];
        brick0_cells[local_idx0].wavelength_nm = 550.0; // green 550nm

        // Put a marker cell at local (5, 6, 7) in brick (1, 1, 0)
        let local_idx1 = emf_brick_voxel_linear_index(5, 6, 7);
        brick1_cells[local_idx1].e_field = [50.0, 0.0, 0.0];
        brick1_cells[local_idx1].b_field = [0.0, 0.0001, 0.0];
        brick1_cells[local_idx1].wavelength_nm = 650.0; // red 650nm

        let h0 = encode_hilbert3d_u64(0, 0, 0, order);
        let h1 = encode_hilbert3d_u64(1, 1, 0, order);

        writer.add_brick_chunk(h0, &brick0_cells, 0, 1, 0).unwrap();
        writer.add_brick_chunk(h1, &brick1_cells, 0, 1, 0).unwrap();

        let bytes = writer.to_bytes().expect("EMF serialize");
        assert_eq!(bytes.len(), writer.total_required_bytes().expect("EMF size"));

        // Parse with zero-copy reader
        let reader = EmfContainerReader::parse(&bytes).expect("valid EMF container");
        assert_eq!(reader.chunk_count(), 2);
        assert_eq!(reader.header().grid_dims_bricks, [2, 2, 2]);
        assert_eq!(reader.header().voxel_size_meters, 0.5);

        // Lookup marker 0
        let cell0 = reader
            .lookup_brick_cell(0, 0, 0, 3, 2, 1, order)
            .expect("found marker 0");
        assert_eq!(cell0.e_field, [100.0, 200.0, 300.0]);
        assert_eq!(cell0.wavelength_nm, 550.0);

        // Lookup marker 1
        let cell1 = reader
            .lookup_brick_cell(1, 1, 0, 5, 6, 7, order)
            .expect("found marker 1");
        assert_eq!(cell1.e_field, [50.0, 0.0, 0.0]);
        assert_eq!(cell1.wavelength_nm, 650.0);

        // Poynting vector & energy density calculation on marker 0
        let poynting = cell0.poynting_vector();
        assert!(poynting[0] != 0.0 || poynting[1] != 0.0 || poynting[2] != 0.0);
        let u_em = cell0.energy_density();
        assert!(u_em > 0.0);

        let dense = reader.dense_volume().expect("dense unpack");
        assert_eq!((dense.nx, dense.ny, dense.nz, dense.nt), (16, 16, 16, 1));
        // brick (0,0,0) local (lx,ly,lz)=(3,2,1) → x=3, y=2, z=1
        let idx0 = (1 * 16 * 16) + (2 * 16) + 3;
        assert_eq!(dense.cells[idx0].e_field, [100.0, 200.0, 300.0]);
        assert_eq!(dense.cells[idx0].wavelength_nm, 550.0);
        assert!((dense.cells[idx0].effective_sigma() - 0.5).abs() < 1e-5);
        // brick (1,1,0) local (5,6,7) → x=13, y=14, z=7
        let idx1 = (7 * 16 * 16) + (14 * 16) + 13;
        assert_eq!(dense.cells[idx1].e_field, [50.0, 0.0, 0.0]);
        assert_eq!(dense.cells[idx1].wavelength_nm, 650.0);
    }

    #[test]
    fn test_emf_tampering_detection() {
        let mut writer = EmfContainerWriter::new([0.0; 3], 1.0, [1, 1, 1], 0.0);
        let cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
        writer.add_brick_chunk(0, &cells, 0, 1, 0).unwrap();

        let mut bytes = writer.to_bytes().expect("EMF serialize");
        assert!(EmfContainerReader::parse(&bytes).is_ok());

        // Tamper with chunk payload byte
        let payload_start = 64 + CHUNK_DIRECTORY_ENTRY_BYTES;
        bytes[payload_start + 10] ^= 0xAA;

        let res = EmfContainerReader::parse(&bytes);
        assert_eq!(
            res.err(),
            Some(EmfError::FullHashMismatch { chunk_index: 0 }),
            "Expected FullHashMismatch when payload is tampered"
        );
    }

    #[test]
    fn test_emf_legacy_52b_upgrade() {
        // Legacy 52-byte tuple: [amp, phase, freq, m0..m9]
        let legacy: [f32; 13] = [
            10.0,       // amp
            1.57,       // phase (pi/2)
            5e14,       // freq 500 THz (~600nm)
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, // 10D manifold
        ];

        let cell = EmfCellRecord64::from_legacy_52b(&legacy);
        assert_eq!(std::mem::size_of::<EmfCellRecord64>(), 64);
        assert_eq!(cell.e_field[0], 10.0);
        assert_eq!(cell.coherence_phase, 1.57);
        assert!(cell.wavelength_nm > 590.0 && cell.wavelength_nm < 610.0);
        assert_eq!(cell.stokes_q, 0.1);
        assert_eq!(cell.stokes_u, 0.2);
        assert_eq!(cell.stokes_v, 0.3);
        assert_eq!(cell.spectral_sigma, 0.4);
        assert_eq!(cell.confidence_alpha, 1.0);
    }

    fn tiny_emf(chunks: u64) -> Vec<u8> {
        let mut writer = EmfContainerWriter::new([0.0; 3], 1.0, [1, 1, 1], 0.0);
        let cells = [EmfCellRecord64::default()];
        for i in 0..chunks {
            writer.add_brick_chunk(i, &cells, 0, 1, 0).unwrap();
        }
        writer.to_bytes().expect("EMF serialize")
    }

    #[test]
    fn writer_size_helpers_reject_wrapping_mul() {
        assert_eq!(
            EmfContainerWriter::checked_payload_bytes(usize::MAX / 64 + 1),
            Err(EmfError::ArithmeticOverflow)
        );
        assert_eq!(
            EmfContainerWriter::checked_directory_bytes(usize::MAX / CHUNK_DIRECTORY_ENTRY_BYTES + 1),
            Err(EmfError::ArithmeticOverflow)
        );
        let empty = EmfContainerWriter::new([0.0; 3], 1.0, [0, 0, 0], 0.0);
        assert_eq!(empty.total_required_bytes().expect("empty EMF size"), 64);
    }

    #[test]
    fn parse_with_limits_enforces_file_and_chunk_budgets() {
        let bytes = tiny_emf(2);
        assert!(EmfContainerReader::parse(&bytes).is_ok());
        assert_eq!(
            EmfContainerReader::parse_with_limits(
                &bytes,
                EmfReadLimits {
                    max_file_bytes: bytes.len() - 1,
                    max_chunks: 16,
                }
            )
            .err(),
            Some(EmfError::FileByteLimit)
        );
        assert_eq!(
            EmfContainerReader::parse_with_limits(
                &bytes,
                EmfReadLimits {
                    max_file_bytes: bytes.len(),
                    max_chunks: 1,
                }
            )
            .err(),
            Some(EmfError::ChunkLimitExceeded)
        );
    }

    #[test]
    fn parse_rejects_directory_aliasing_header_and_payload_overlap() {
        let mut bytes = tiny_emf(2);
        let mut header: EmfContainerHeader = bytemuck::pod_read_unaligned(&bytes[..64]);
        header.directory_offset = 0;
        header.update_crc();
        bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
        assert_eq!(
            EmfContainerReader::parse(&bytes).err(),
            Some(EmfError::DirectoryBounds)
        );

        let mut bytes = tiny_emf(2);
        let dir1 = 64 + CHUNK_DIRECTORY_ENTRY_BYTES;
        let mut entry: EmfChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&bytes[dir1..dir1 + CHUNK_DIRECTORY_ENTRY_BYTES]);
        entry.byte_offset = 192;
        entry.update_crc();
        bytes[dir1..dir1 + CHUNK_DIRECTORY_ENTRY_BYTES].copy_from_slice(bytemuck::bytes_of(&entry));
        assert_eq!(
            EmfContainerReader::parse(&bytes).err(),
            Some(EmfError::PayloadOverlap)
        );
    }

    #[test]
    fn dense_volume_rejects_decoded_cell_cap() {
        // 17×16×8 bricks × 8³ cells = 1_114_112 > MAX_DENSE_CELLS (1_048_576).
        // One tiny payload still parses; dense unpack must fail closed before vec![].
        let mut writer = EmfContainerWriter::new([0.0; 3], 1.0, [17, 16, 8], 0.0);
        let cells = [EmfCellRecord64::default()];
        writer.add_brick_chunk(0, &cells, 0, 1, 0).unwrap();
        let bytes = writer.to_bytes().expect("EMF serialize");
        let reader = EmfContainerReader::parse(&bytes).expect("tiny payload parses");
        let err = reader.dense_volume().expect_err("dense volume cap");
        assert_eq!(
            err,
            EmfError::DenseVolumeCap {
                cells: 1_114_112,
                cap: MAX_DENSE_CELLS,
            }
        );
        assert_eq!(
            err.to_string(),
            "EMF dense volume 1114112 cells exceeds cap 1048576"
        );
    }
}
