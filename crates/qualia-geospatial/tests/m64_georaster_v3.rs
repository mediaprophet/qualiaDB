use qualia_core_db::container_10d::crc32c::crc32c;
use qualia_core_db::q42::m64_matter::*;
use qualia_geospatial::raster_m64::*;
use qualia_geospatial::{compile_raster_to_m64, ElevationRaster, GeodeticFrame};
use std::sync::atomic::AtomicBool;

struct CountingAllocator;
thread_local! {
    static MEASURING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if MEASURING.try_with(|v| v.get()).unwrap_or(false) {
            let _ = ALLOCATIONS.try_with(|v| v.set(v.get() + 1));
        }
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        if MEASURING.try_with(|v| v.get()).unwrap_or(false) {
            let _ = ALLOCATIONS.try_with(|v| v.set(v.get() + 1));
        }
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, ptr, layout, size) }
    }
}

fn policy() -> RasterM64Policy {
    RasterM64Policy {
        rows: M64RasterRows::South,
        edges: M64RasterEdges::RepeatNearest,
        nodata_sentinel_mm: Some(i32::MIN),
    }
}
fn limits() -> RasterM64Limits {
    RasterM64Limits {
        max_output_bytes: 2_000_000,
        max_chunks: 4,
        max_records: 16_384,
    }
}
fn frame() -> GeodeticFrame {
    GeodeticFrame {
        origin_lat_deg: -35.7812345678901,
        origin_lon_deg: 148.298765432109,
        origin_elevation_mm: 16_777_217,
        epoch_year: 2020.25,
        ..GeodeticFrame::default()
    }
}
fn fixture(width: usize, height: usize) -> (Vec<u8>, Vec<i32>) {
    let mut values: Vec<i32> = (0..width * height).map(|n| n as i32 * 31 - 900).collect();
    values[0] = i32::MIN;
    let raster = ElevationRaster {
        width,
        height,
        cell_size_mm: 30_001,
        elevations_mm: &values,
    };
    let mut bytes = vec![0xa5; 2_000_000];
    let count = compile_raster_to_m64_v3(
        &raster,
        &frame(),
        7,
        policy(),
        limits(),
        &AtomicBool::new(false),
        &mut bytes,
    )
    .unwrap();
    bytes.truncate(count);
    (bytes, values)
}
fn parse(bytes: &[u8]) -> Result<M64Reader<'_>, M64ReadError> {
    M64Reader::parse(
        bytes,
        M64ReadLimits {
            max_file_bytes: 2_000_000,
            max_chunks: 4,
            max_records: 16_384,
        },
        &mut [M64PayloadInterval::default(); 4],
    )
}
fn metadata_crc(bytes: &mut [u8]) {
    let crc = crc32c(&bytes[64..188]);
    bytes[188..192].copy_from_slice(&crc.to_le_bytes());
}
fn header_mutate(bytes: &mut [u8], f: impl FnOnce(&mut M64Header)) {
    let mut header: M64Header = bytemuck::pod_read_unaligned(&bytes[..64]);
    f(&mut header);
    header.update_crc();
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
}
fn entry_mutate(bytes: &mut [u8], chunk: usize, f: impl FnOnce(&mut M64ChunkDirectoryEntry)) {
    let offset = 192 + chunk * 64;
    let mut entry: M64ChunkDirectoryEntry =
        bytemuck::pod_read_unaligned(&bytes[offset..offset + 64]);
    f(&mut entry);
    entry.update_crc();
    bytes[offset..offset + 64].copy_from_slice(bytemuck::bytes_of(&entry));
}

#[test]
fn v3_lossless_metadata_samples_nodata_and_padding() {
    let (bytes, values) = fixture(65, 2);
    let reader = parse(&bytes).unwrap();
    let meta = reader.georaster().unwrap();
    assert_eq!(reader.header().version, 3);
    assert_eq!(
        meta.origin_lat_deg.to_bits(),
        frame().origin_lat_deg.to_bits()
    );
    assert_eq!(
        meta.origin_lon_deg.to_bits(),
        frame().origin_lon_deg.to_bits()
    );
    assert_eq!(meta.epoch_year.to_bits(), frame().epoch_year.to_bits());
    assert_eq!(
        (meta.width, meta.height, meta.cell_size_mm),
        (65, 2, 30_001)
    );
    assert_eq!(meta.origin_elevation_mm, 16_777_217);
    assert_eq!(meta.vertical_datum, frame().vertical_datum);
    assert_eq!(meta.rows, M64RasterRows::South);
    assert_eq!(
        reader.raster_sample(0, 0),
        Some(M64RasterSample::Nodata(i32::MIN))
    );
    assert_eq!(
        reader.raster_sample(64, 1),
        Some(M64RasterSample::ElevationMm(values[129]))
    );
    assert_eq!(reader.raster_sample(65, 0), None);
    assert_eq!(reader.raster_sample(0, 2), None);
    let mut recovered = vec![0; values.len() + 1];
    recovered[values.len()] = 777;
    let (count, recovered_frame) =
        decode_raster_from_m64_v3(&reader, &AtomicBool::new(false), &mut recovered).unwrap();
    assert_eq!(&recovered[..count], values);
    assert_eq!(recovered[count], 777);
    assert_eq!(recovered_frame, frame());
    assert_eq!(reader.verification().record_crc_checked, 8192);
}

#[test]
fn v3_deterministic_and_unaligned_input() {
    let (bytes, _) = fixture(3, 3);
    assert_eq!(bytes, fixture(3, 3).0);
    let mut unaligned = vec![0];
    unaligned.extend_from_slice(&bytes);
    assert!(parse(&unaligned[1..]).is_ok());
}

#[test]
fn legacy_versions_defaults_and_e64_are_preserved() {
    assert_eq!(M64_VERSION, 2);
    assert_eq!(M64Header::default().version, 2);
    let values = [123];
    let raster = ElevationRaster {
        width: 1,
        height: 1,
        cell_size_mm: 30_000,
        elevations_mm: &values,
    };
    let mut bytes = vec![0; 64 + 64 + 4096 * 64];
    compile_raster_to_m64(&raster, &frame(), 1, &mut bytes).unwrap();
    let reader = parse(&bytes).unwrap();
    assert_eq!(reader.header().version, 1);
    assert!(reader.georaster().is_none());
    assert_eq!(reader.raster_sample(0, 0), None);
    header_mutate(&mut bytes, |h| {
        h.version = 2;
        h.magic = E64_MAGIC;
    });
    assert!(parse(&bytes).is_ok());
    // V3 requires an explicit enlarged header, not relabelled legacy bytes.
    header_mutate(&mut bytes, |h| h.version = 3);
    assert!(matches!(
        parse(&bytes),
        Err(M64ReadError::Header(M64Error::InvalidHeaderSize))
    ));
}

#[test]
fn metadata_crc_reserved_and_counts_are_checked() {
    let (original, _) = fixture(1, 1);
    let mut bad = original.clone();
    bad[88] ^= 1;
    assert!(matches!(
        parse(&bad),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::Checksum))
    ));
    for offset in [127, 160] {
        // reserved byte 63 and reserved tail
        let mut bad = original.clone();
        bad[offset] = 1;
        metadata_crc(&mut bad);
        assert!(matches!(
            parse(&bad),
            Err(M64ReadError::GeoRaster(M64GeoRasterError::Reserved))
        ));
    }
    let mut bad = original.clone();
    bad[136] ^= 1; // emitted record count
    metadata_crc(&mut bad);
    assert!(matches!(
        parse(&bad),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::Counts))
    ));
}

#[test]
fn metadata_policy_and_none_sentinel_refuse_unsupported_semantics() {
    let (original, _) = fixture(1, 1);
    for offset in [122, 123, 124, 125, 126] {
        // row/anchor/basis/edge/nodata tags
        let mut bad = original.clone();
        bad[offset] = 9;
        metadata_crc(&mut bad);
        assert!(matches!(
            parse(&bad),
            Err(M64ReadError::GeoRaster(M64GeoRasterError::Policy))
        ));
    }
    let mut bad = original;
    bad[126] = 0; // nodata none while sentinel remains i32::MIN
    metadata_crc(&mut bad);
    assert!(matches!(
        parse(&bad),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::Reserved))
    ));
}

#[test]
fn header_summaries_and_v3_magic_must_agree() {
    let (original, _) = fixture(1, 1);
    for field in 0..4 {
        let mut bad = original.clone();
        header_mutate(&mut bad, |h| match field {
            0 => h.spatial_origin[0] += 1.0,
            1 => h.elevation_datum[0] += 1.0,
            2 => h.elevation_datum[1] = 2.0,
            _ => h.magic = E64_MAGIC,
        });
        assert!(matches!(
            parse(&bad),
            Err(M64ReadError::GeoRaster(M64GeoRasterError::HeaderSummary))
        ));
    }
}

#[test]
fn row_major_directory_srid_codec_bao_mode_and_extents_are_checked() {
    let (original, _) = fixture(65, 2);
    for field in 0..6 {
        let mut bad = original.clone();
        entry_mutate(&mut bad, 0, |entry| match field {
            0 => entry.srid_geodetic_code += 1,
            1 => entry.spatial_hilbert_index += 1,
            2 => entry.bao_root_hash_prefix[0] = 1,
            3 => entry.compression_flags = 1,
            4 => entry.chunk_record_mode = M64_MODE_POINT,
            _ => entry.byte_offset += 64,
        });
        assert!(parse(&bad).is_err());
    }
}

#[test]
fn padding_is_verified_after_valid_record_crc() {
    let (mut bad, _) = fixture(1, 1);
    let start = 192 + 64 + 64; // second record is outside source extent
    let mut record: M64StrataColumnRecord = bytemuck::pod_read_unaligned(&bad[start..start + 64]);
    record.surface_elevation_mm = 111;
    record.update_crc();
    bad[start..start + 64].copy_from_slice(bytemuck::bytes_of(&record));
    assert!(matches!(
        parse(&bad),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::Padding))
    ));
}

#[test]
fn raster_profile_refuses_fabricated_physical_fields() {
    let (mut bad, _) = fixture(1, 1);
    let start = 192 + 64;
    let mut record: M64StrataColumnRecord = bytemuck::pod_read_unaligned(&bad[start..start + 64]);
    record.water_table_depth_mm = 100;
    record.update_crc();
    bad[start..start + 64].copy_from_slice(bytemuck::bytes_of(&record));
    assert!(matches!(
        parse(&bad),
        Err(M64ReadError::GeoRaster(M64GeoRasterError::NonRasterFields))
    ));
}

#[test]
fn writer_limits_cancellation_and_validation_leave_output_unchanged() {
    let values = [77];
    let raster = ElevationRaster {
        width: 1,
        height: 1,
        cell_size_mm: 1,
        elevations_mm: &values,
    };
    for field in 0..6 {
        let mut output = vec![0xa5; 300_000];
        let mut budget = limits();
        let mut selected = policy();
        let mut origin = frame();
        match field {
            0 => budget.max_output_bytes = 1,
            1 => budget.max_chunks = 0,
            2 => budget.max_records = 4095,
            3 => selected.edges = M64RasterEdges::RejectPartial,
            4 => origin.origin_lat_deg = f64::NAN,
            _ => {}
        }
        let result = compile_raster_to_m64_v3(
            &raster,
            &origin,
            1,
            selected,
            budget,
            &AtomicBool::new(field == 5),
            &mut output,
        );
        assert!(result.is_err());
        assert!(output.iter().all(|&v| v == 0xa5));
    }
    let mut short = [0xa5; 192];
    assert_eq!(
        compile_raster_to_m64_v3(
            &raster,
            &frame(),
            1,
            policy(),
            limits(),
            &AtomicBool::new(false),
            &mut short
        ),
        Err(RasterM64Error::BufferTooSmall)
    );
    assert_eq!(short, [0xa5; 192]);
}

#[test]
fn metadata_admission_bounds_and_full_chunk_no_nodata() {
    let (bytes, values) = fixture(64, 64);
    let reader = parse(&bytes).unwrap();
    let mut meta = *reader.georaster().unwrap();
    for field in 0..7 {
        let mut bad = meta;
        match field {
            0 => bad.cell_size_mm = 0,
            1 => bad.width = 0,
            2 => bad.width = u64::MAX,
            3 => bad.origin_lat_deg = 91.0,
            4 => bad.origin_lon_deg = f64::INFINITY,
            5 => bad.epoch_year = f32::NAN,
            _ => bad.height = 65_536 * 64 + 1,
        }
        let mut block = [0xa5; 128];
        assert!(bad.encode(&mut block).is_err());
        assert_eq!(block, [0xa5; 128]);
    }
    meta.edges = M64RasterEdges::RejectPartial;
    meta.rows = M64RasterRows::North;
    meta.nodata_sentinel_mm = None;
    let raster = ElevationRaster {
        width: 64,
        height: 64,
        cell_size_mm: meta.cell_size_mm,
        elevations_mm: &values,
    };
    let mut out = vec![0; bytes.len()];
    let mut selected = policy();
    selected.edges = meta.edges;
    selected.rows = meta.rows;
    selected.nodata_sentinel_mm = None;
    compile_raster_to_m64_v3(
        &raster,
        &frame(),
        7,
        selected,
        limits(),
        &AtomicBool::new(false),
        &mut out,
    )
    .unwrap();
    assert_eq!(
        parse(&out).unwrap().raster_sample(0, 0),
        Some(M64RasterSample::ElevationMm(i32::MIN))
    );
}

#[test]
fn reader_limits_and_decode_refusals_are_explicit() {
    let (bytes, _) = fixture(65, 2);
    for field in 0..3 {
        let mut budget = M64ReadLimits {
            max_file_bytes: 2_000_000,
            max_chunks: 4,
            max_records: 16_384,
        };
        match field {
            0 => budget.max_file_bytes = 1,
            1 => budget.max_chunks = 1,
            _ => budget.max_records = 8191,
        }
        assert!(M64Reader::parse(&bytes, budget, &mut [M64PayloadInterval::default(); 4]).is_err());
    }
    let reader = parse(&bytes).unwrap();
    let mut out = [99; 130];
    assert_eq!(
        decode_raster_from_m64_v3(&reader, &AtomicBool::new(true), &mut out),
        Err(RasterM64Error::Cancelled)
    );
    assert_eq!(out, [99; 130]);
    let mut short = [99; 1];
    assert_eq!(
        decode_raster_from_m64_v3(&reader, &AtomicBool::new(false), &mut short),
        Err(RasterM64Error::BufferTooSmall)
    );
    assert_eq!(short, [99; 1]);
}

#[test]
fn v3_writer_reader_and_recovery_use_caller_buffers_without_heap() {
    let (bytes, values) = fixture(65, 2); // warm CRC table before measurement
    let raster = ElevationRaster {
        width: 65,
        height: 2,
        cell_size_mm: 30_001,
        elevations_mm: &values,
    };
    let mut out = vec![0; bytes.len()];
    let mut recovered = vec![0; values.len()];
    let mut scratch = [M64PayloadInterval::default(); 4];
    ALLOCATIONS.with(|n| n.set(0));
    MEASURING.with(|flag| flag.set(true));
    let written = compile_raster_to_m64_v3(
        &raster,
        &frame(),
        7,
        policy(),
        limits(),
        &AtomicBool::new(false),
        &mut out,
    );
    let read = M64Reader::parse(
        &out,
        M64ReadLimits {
            max_file_bytes: 2_000_000,
            max_chunks: 4,
            max_records: 16_384,
        },
        &mut scratch,
    );
    let decoded = read
        .as_ref()
        .map(|reader| decode_raster_from_m64_v3(reader, &AtomicBool::new(false), &mut recovered));
    MEASURING.with(|flag| flag.set(false));
    assert!(written.is_ok());
    assert!(read.is_ok());
    assert!(decoded.unwrap().is_ok());
    assert_eq!(recovered, values);
    assert_eq!(ALLOCATIONS.with(|n| n.get()), 0);
}
