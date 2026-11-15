use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{
        encode_proof, sibling_ranges, verify_leaf_span, OutboardNode, ProofSiblingRange,
        SpanLimits, TreeBuilder, MAX_SIBLINGS, LEAF_BYTES,
    },
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::m64_matter::{
    georaster::{
        georaster_chunk_hilbert, M64GeoRasterError, M64GeoRasterMetadata, M64RasterEdges,
        M64RasterRows,
    },
    M64ChunkDirectoryEntry, M64Header, M64PayloadInterval, M64RasterSample, M64ReadError,
    M64ReadLimits, M64Reader, M64StrataColumnRecord, M64_FLAG_LOCAL_METRIC_LRM3, M64_MODE_STRATA,
};
use bytemuck::Zeroable;

const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const CHUNK_BYTES: usize = 4096 * 64;
const DIRECTORY_OFFSET: usize = 192;
const DIRECTORY_BYTES: usize = 2 * 64;
const PAYLOAD_OFFSET: usize = DIRECTORY_OFFSET + DIRECTORY_BYTES;

fn metadata() -> M64GeoRasterMetadata {
    M64GeoRasterMetadata {
        width: WIDTH as u64,
        height: HEIGHT as u64,
        origin_lat_deg: 1.5,
        origin_lon_deg: 2.5,
        origin_elevation_mm: 1000,
        cell_size_mm: 250,
        epoch_year: 2024.0,
        srid: 4326,
        vertical_datum: 1,
        rows: M64RasterRows::North,
        edges: M64RasterEdges::RejectPartial,
        nodata_sentinel_mm: Some(i32::MIN),
    }
}

fn write_header(bytes: &mut [u8], meta: &M64GeoRasterMetadata, chunks: u64, total: u64) {
    let mut header = M64Header::default();
    header.version = 3;
    header.header_size = 192;
    header.total_file_size = total;
    header.spatial_origin = [meta.origin_lat_deg as f32, meta.origin_lon_deg as f32];
    header.elevation_datum = [meta.origin_elevation_mm as f32 / 1000.0, 1.0];
    header.chunk_table_offset = DIRECTORY_OFFSET as u64;
    header.chunk_count = chunks;
    header.container_flags = 0;
    header.update_crc();
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
}

fn write_directory_row(
    bytes: &mut [u8],
    meta: &M64GeoRasterMetadata,
    chunk: usize,
    chunks_x: usize,
    payload_start: usize,
) {
    let x = (chunk % chunks_x) as u32;
    let y = (chunk / chunks_x) as u32;
    let mut row = M64ChunkDirectoryEntry {
        chunk_uuid: [chunk as u8 + 1; 16],
        spatial_hilbert_index: georaster_chunk_hilbert(x, y),
        byte_offset: (payload_start + chunk * CHUNK_BYTES) as u64,
        byte_length: CHUNK_BYTES as u32,
        record_count: 4096,
        srid_geodetic_code: meta.srid,
        lut_bank_epoch_id: 0,
        chunk_record_mode: M64_MODE_STRATA,
        compression_flags: 0,
        bao_root_hash_prefix: [0; 12],
        entry_crc32c: 0,
    };
    row.update_crc();
    let dir_start = DIRECTORY_OFFSET + chunk * 64;
    bytes[dir_start..dir_start + 64].copy_from_slice(bytemuck::bytes_of(&row));
}

fn write_record(bytes: &mut [u8], offset: usize, elevation: i32) {
    let mut record = M64StrataColumnRecord::zeroed();
    record.surface_elevation_mm = elevation;
    record.update_crc();
    bytes[offset..offset + 64].copy_from_slice(bytemuck::bytes_of(&record));
}

fn m64_v3_bytes() -> Vec<u8> {
    let meta = metadata();
    let total = PAYLOAD_OFFSET + 2 * CHUNK_BYTES;
    let mut bytes = vec![0; total];
    write_header(&mut bytes, &meta, 2, total as u64);
    meta.encode(&mut bytes[64..192]).unwrap();
    for chunk in 0..2 {
        write_directory_row(&mut bytes, &meta, chunk, 2, PAYLOAD_OFFSET);
        for record_index in 0..4096 {
            let start = PAYLOAD_OFFSET + chunk * CHUNK_BYTES + record_index * 64;
            write_record(&mut bytes, start, (chunk * 4096 + record_index) as i32);
        }
    }
    bytes
}

fn repeat_nearest_bytes() -> Vec<u8> {
    let mut meta = metadata();
    meta.width = 65;
    meta.height = 64;
    meta.edges = M64RasterEdges::RepeatNearest;
    let shape = meta.shape().unwrap();
    assert_eq!(shape.chunks, 2);
    let mut bytes = vec![0; shape.file_bytes];
    write_header(&mut bytes, &meta, 2, shape.file_bytes as u64);
    meta.encode(&mut bytes[64..192]).unwrap();
    let payload_start = DIRECTORY_OFFSET + shape.chunks * 64;
    for chunk in 0..shape.chunks {
        write_directory_row(&mut bytes, &meta, chunk, shape.chunks_x, payload_start);
        for record_index in 0..4096 {
            let x = (chunk % shape.chunks_x) * 64 + record_index % 64;
            let y = (chunk / shape.chunks_x) * 64 + record_index / 64;
            let sx = x.min(shape.width - 1);
            let sy = y.min(shape.height - 1);
            let start = payload_start + chunk * CHUNK_BYTES + record_index * 64;
            write_record(&mut bytes, start, (sy * 65 + sx) as i32);
        }
    }
    bytes
}

fn leaf_span<'a>(
    selected: SelectedPlainEntry<'_>,
    bytes: &'a [u8],
    leaf: u64,
) -> crate::bundle::stream::proof::VerifiedLeafSpan<'a> {
    let spec = selected.tree_spec().unwrap();
    let mut tree = TreeBuilder::new(spec);
    let mut nodes = Vec::<OutboardNode>::new();
    for (index, chunk) in bytes.chunks(LEAF_BYTES as usize).enumerate() {
        tree.append(chunk, |node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
        assert_eq!(index as u64, tree.appended() - 1);
    }
    assert_eq!(
        tree.finish(|node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap(),
        selected.root()
    );

    let mut path = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
    let count = sibling_ranges(spec.leaf_count(), leaf, &mut path).unwrap();
    let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
    for (target, range) in siblings.iter_mut().zip(&path[..count]) {
        *target = nodes
            .iter()
            .find(|n| n.start_leaf == range.start_leaf && n.leaf_count == range.leaf_count)
            .unwrap()
            .digest;
    }
    let mut encoded = [0u8; 1537];
    let encoded_len = encode_proof(&siblings[..count], &mut encoded).unwrap();
    let start = usize::try_from(leaf * LEAF_BYTES).unwrap();
    let end = (start + LEAF_BYTES as usize).min(bytes.len());
    let mut scratch = [StrongDigest::ZERO; MAX_SIBLINGS];
    verify_leaf_span(
        &spec,
        selected.root(),
        leaf,
        &bytes[start..end],
        &[&encoded[..encoded_len]],
        &mut scratch,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: LEAF_BYTES as usize,
        },
    )
    .unwrap()
}

fn sample_offset(x: usize, y: usize) -> usize {
    let chunk = (y / 64) * 2 + x / 64;
    let record = (y % 64) * 64 + x % 64;
    PAYLOAD_OFFSET + chunk * CHUNK_BYTES + record * 64
}

fn parse_limits(bytes: &[u8]) -> M64ReadLimits {
    M64ReadLimits {
        max_file_bytes: bytes.len(),
        max_chunks: 2,
        max_records: 8192,
    }
}

// The ranges are backed by actual selected-entry proofs; fixture/test ownership is cold setup.
macro_rules! with_ranges {
    ($bytes:expr, $offset:expr, |$header:ident, $meta:ident, $directory:ident, $sample:ident| $body:block) => {{
        let bytes = $bytes;
        let fixture = Fixture::new_with_payload(&bytes);
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let closure = fixture.bound(&pins, pin, publisher);
        let qbdl_header = admit_header(
            &fixture.source()[..64],
            fixture.source().len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let mut entry_scratch = [EntryScratch::EMPTY; 1];
        let index = admit_index(
            &qbdl_header,
            &fixture.source()[qbdl_header.index_offset() as usize..],
            IndexReadLimits::default(),
            &mut entry_scratch,
        )
        .unwrap();
        let mut rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        let selected = select_entry(
            &closure,
            fixture.body(),
            &fixture.source()[..64],
            &index,
            fixture.entry_page(),
            0,
            &mut rows,
            &mut framing,
        )
        .unwrap();
        let metadata_span = leaf_span(selected, &bytes, 0);
        let $header = AuthenticatedRange::new(selected, &metadata_span, 0, 64).unwrap();
        let $meta = AuthenticatedRange::new(selected, &metadata_span, 64, 128).unwrap();
        let $directory =
            AuthenticatedRange::new(selected, &metadata_span, 192, DIRECTORY_BYTES).unwrap();
        let offset = $offset;
        let sample_span = leaf_span(selected, &bytes, (offset / LEAF_BYTES as usize) as u64);
        let $sample = AuthenticatedRange::new(selected, &sample_span, offset as u64, 64).unwrap();
        $body
    }};
}

fn limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 1024 * 1024,
        max_rows: 4,
        max_metadata_bytes: 1024,
        max_chunk_bytes: 64,
    }
}

#[test]
fn m64_v3_reads_one_proved_in_bounds_sample_without_a_chunk_view() {
    let bytes = m64_v3_bytes();
    let mut full_scratch = [M64PayloadInterval::default(); 2];
    let full = M64Reader::parse(&bytes, parse_limits(&bytes), &mut full_scratch).unwrap();
    assert_eq!(
        full.raster_sample(65, 8),
        Some(M64RasterSample::ElevationMm(4609))
    );

    let offset = sample_offset(65, 8);
    with_ranges!(&bytes, offset, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert_eq!(view.header().version, 3);
        assert_eq!(
            view.selected_sample(65, 8, p).unwrap(),
            M64RasterSample::ElevationMm(4609)
        );
        assert_eq!(p.bytes().len(), 64);
        assert_eq!(p.bytes().as_ptr(), bytes[offset..].as_ptr());
        assert!(matches!(
            view.selected_sample(128, 8, p),
            Err(FormatError::Geometry)
        ));
        assert!(matches!(
            view.selected_sample(65, 64, p),
            Err(FormatError::Geometry)
        ));
        let leaf = leaf_span(
            p.selected(),
            &bytes,
            (offset / LEAF_BYTES as usize) as u64,
        );
        let shifted = AuthenticatedRange::new(
            p.selected(),
            &leaf,
            offset as u64 + 64,
            64,
        )
        .unwrap();
        assert!(matches!(
            view.selected_sample(65, 8, shifted),
            Err(FormatError::Bounds)
        ));
        let short = AuthenticatedRange::new(
            p.selected(),
            &leaf,
            offset as u64,
            63,
        )
        .unwrap();
        assert!(matches!(
            view.selected_sample(65, 8, short),
            Err(FormatError::Bounds)
        ));
    });
}

#[test]
fn m64_v3_metadata_directory_and_sample_fail_closed() {
    let valid = m64_v3_bytes();
    let offset = sample_offset(65, 8);
    with_ranges!(&valid, offset, |h, g, d, p| {
        assert!(M64GeoRasterDirectoryView::admit(
            h,
            g,
            d,
            FormatLimits {
                max_rows: 1,
                ..limits()
            },
            8192
        )
        .is_err());
        assert!(M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8191).is_err());
        assert!(M64GeoRasterDirectoryView::admit(
            h,
            g,
            d,
            FormatLimits {
                max_metadata_bytes: 319,
                ..limits()
            },
            8192
        )
        .is_err());
        assert!(M64GeoRasterDirectoryView::admit(
            h,
            g,
            d,
            FormatLimits {
                max_chunk_bytes: 63,
                ..limits()
            },
            8192
        )
        .is_err());
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert!(M64GeoRasterDirectoryView::admit(
            h,
            AuthenticatedRange::new(p.selected(), &leaf_span(p.selected(), &valid, 0), 65, 128)
                .unwrap(),
            d,
            limits(),
            8192
        )
        .is_err());
        assert!(M64GeoRasterDirectoryView::admit(
            h,
            g,
            AuthenticatedRange::new(p.selected(), &leaf_span(p.selected(), &valid, 0), 193, 128)
                .unwrap(),
            limits(),
            8192
        )
        .is_err());
        assert!(matches!(
            view.selected_sample(
                65,
                8,
                AuthenticatedRange::new(p.selected(), &leaf_span(p.selected(), &valid, 0), 0, 64)
                    .unwrap()
            ),
            Err(FormatError::Bounds)
        ));
    });

    let mut bad_summary = valid.clone();
    let mut header: M64Header = bytemuck::pod_read_unaligned(&bad_summary[..64]);
    header.spatial_origin[0] = 9.0;
    header.update_crc();
    bad_summary[..64].copy_from_slice(bytemuck::bytes_of(&header));
    with_ranges!(&bad_summary, offset, |h, g, d, _p| {
        assert!(matches!(
            M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192),
            Err(FormatError::Geometry)
        ));
    });

    let mut bad_version = valid.clone();
    let mut header: M64Header = bytemuck::pod_read_unaligned(&bad_version[..64]);
    header.version = 2;
    header.header_size = 64;
    header.update_crc();
    bad_version[..64].copy_from_slice(bytemuck::bytes_of(&header));
    with_ranges!(&bad_version, offset, |h, g, d, _p| {
        assert!(matches!(
            M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192),
            Err(FormatError::Unsupported)
        ));
    });

    let mut bad_flags = valid.clone();
    let mut header: M64Header = bytemuck::pod_read_unaligned(&bad_flags[..64]);
    header.container_flags = M64_FLAG_LOCAL_METRIC_LRM3 as u32;
    header.update_crc();
    bad_flags[..64].copy_from_slice(bytemuck::bytes_of(&header));
    with_ranges!(&bad_flags, offset, |h, g, d, _p| {
        assert!(matches!(
            M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192),
            Err(FormatError::Unsupported)
        ));
    });

    let mut bad_grm3 = valid.clone();
    bad_grm3[70] ^= 1;
    with_ranges!(&bad_grm3, offset, |h, g, d, _p| {
        assert!(matches!(
            M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192),
            Err(FormatError::Integrity)
        ));
    });

    let mut bad_row = valid.clone();
    let mut row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bad_row[256..320]);
    row.spatial_hilbert_index ^= 1;
    row.update_crc();
    bad_row[256..320].copy_from_slice(bytemuck::bytes_of(&row));
    with_ranges!(&bad_row, offset, |h, g, d, _p| {
        assert!(M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).is_err());
    });

    let mut bad_offset = valid.clone();
    let mut row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bad_offset[192..256]);
    row.byte_offset += 64;
    row.update_crc();
    bad_offset[192..256].copy_from_slice(bytemuck::bytes_of(&row));
    with_ranges!(&bad_offset, offset, |h, g, d, _p| {
        assert!(M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).is_err());
    });

    let mut bad_order = valid.clone();
    let first = bad_order[192..256].to_vec();
    let second = bad_order[256..320].to_vec();
    bad_order[192..256].copy_from_slice(&second);
    bad_order[256..320].copy_from_slice(&first);
    with_ranges!(&bad_order, offset, |h, g, d, _p| {
        assert!(M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).is_err());
    });

    let mut bad_row_crc = valid.clone();
    bad_row_crc[192] ^= 1;
    with_ranges!(&bad_row_crc, offset, |h, g, d, _p| {
        assert!(M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).is_err());
    });

    let mut bad_record = valid.clone();
    bad_record[offset + 5] = 1;
    let mut record: M64StrataColumnRecord =
        bytemuck::pod_read_unaligned(&bad_record[offset..offset + 64]);
    record.update_crc();
    bad_record[offset..offset + 64].copy_from_slice(bytemuck::bytes_of(&record));
    with_ranges!(&bad_record, offset, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert!(matches!(
            view.selected_sample(65, 8, p),
            Err(FormatError::Integrity)
        ));
    });

    let mut bad_crc = valid.clone();
    bad_crc[offset + 60] ^= 1;
    with_ranges!(&bad_crc, offset, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert!(matches!(
            view.selected_sample(65, 8, p),
            Err(FormatError::Integrity)
        ));
    });
}

#[test]
fn m64_v3_rejects_a_range_from_another_selected_entry() {
    let original = m64_v3_bytes();
    let mut other = original.clone();
    other[sample_offset(65, 8)] ^= 1;
    let offset = sample_offset(65, 8);
    with_ranges!(&original, offset, |h, g, d, _p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        with_ranges!(&other, offset, |_h2, _g2, _d2, other_sample| {
            assert!(matches!(
                view.selected_sample(65, 8, other_sample),
                Err(FormatError::Identity)
            ));
        });
    });
}

#[test]
fn m64_v3_admits_an_unaffected_sample_when_an_unfetched_chunk_is_corrupt() {
    let mut bytes = m64_v3_bytes();
    let corrupt = PAYLOAD_OFFSET;
    bytes[corrupt + 5] = 1;
    let mut record: M64StrataColumnRecord =
        bytemuck::pod_read_unaligned(&bytes[corrupt..corrupt + 64]);
    record.update_crc();
    bytes[corrupt..corrupt + 64].copy_from_slice(bytemuck::bytes_of(&record));

    let mut scratch = [M64PayloadInterval::default(); 2];
    assert!(matches!(
        M64Reader::parse(&bytes, parse_limits(&bytes), &mut scratch),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::NonRasterFields))
    ));

    let offset = sample_offset(65, 8);
    with_ranges!(&bytes, offset, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert_eq!(
            view.selected_sample(65, 8, p).unwrap(),
            M64RasterSample::ElevationMm(4609)
        );
        // Fresh outer entry root authenticates this selected record only.
        // Chunk 0 was not fetched and is not validated by this partial view.
    });
}

#[test]
fn m64_v3_repeat_nearest_exposes_in_bounds_samples_not_padded_cells() {
    let bytes = repeat_nearest_bytes();
    let mut scratch = [M64PayloadInterval::default(); 2];
    let full = M64Reader::parse(&bytes, parse_limits(&bytes), &mut scratch).unwrap();
    assert_eq!(
        full.raster_sample(64, 8),
        Some(M64RasterSample::ElevationMm(8 * 65 + 64))
    );
    assert_eq!(full.raster_sample(65, 8), None);

    let in_bounds = sample_offset(64, 8);
    with_ranges!(&bytes, in_bounds, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert_eq!(
            view.selected_sample(64, 8, p).unwrap(),
            M64RasterSample::ElevationMm(8 * 65 + 64)
        );
        assert!(matches!(
            view.selected_sample(65, 8, p),
            Err(FormatError::Geometry)
        ));
    });

    let padded = sample_offset(65, 8);
    with_ranges!(&bytes, padded, |h, g, d, p| {
        let view = M64GeoRasterDirectoryView::admit(h, g, d, limits(), 8192).unwrap();
        assert_eq!(p.bytes().as_ptr(), bytes[padded..].as_ptr());
        assert!(matches!(
            view.selected_sample(65, 8, p),
            Err(FormatError::Geometry)
        ));
        assert!(matches!(
            view.selected_sample(64, 8, p),
            Err(FormatError::Bounds)
        ));
    });
}
