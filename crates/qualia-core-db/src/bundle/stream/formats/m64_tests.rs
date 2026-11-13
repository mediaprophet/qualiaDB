use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits},
    selection::select_entry,
};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::{
    chunk_integrity::blake3_trailer,
    m64_matter::{
        M64ChunkDirectoryEntry, M64Header, M64LocalMetricHeader, M64StrataColumnRecord, E64_MAGIC,
        M64_CODEC_BLAKE3_FULL, M64_FLAG_LOCAL_METRIC_LRM3, M64_MODE_STRATA,
    },
};

fn m64_limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 16_384,
        max_rows: 4,
        max_metadata_bytes: 512,
        max_chunk_bytes: 256,
    }
}
fn m64_bytes(with_metric: bool, magic: [u8; 4], full_codec: bool) -> Vec<u8> {
    let header_size = 64usize;
    let metric_len = if with_metric { 128 } else { 0 };
    let dir = header_size + metric_len;
    let payload_off = dir + 64;
    let rec_end = payload_off + 64;
    let total = rec_end + if full_codec { 64 } else { 0 };
    let mut rec = M64StrataColumnRecord::default();
    rec.surface_elevation_mm = 1234;
    rec.update_crc();
    let record = bytemuck::bytes_of(&rec);
    let mut header = M64Header::default();
    header.magic = magic;
    header.total_file_size = total as u64;
    header.chunk_count = 1;
    header.chunk_table_offset = dir as u64;
    header.container_flags = if with_metric {
        M64_FLAG_LOCAL_METRIC_LRM3 as u32
    } else {
        0
    };
    header.update_crc();
    let mut row = M64ChunkDirectoryEntry {
        chunk_uuid: [0x31; 16],
        spatial_hilbert_index: 7,
        byte_offset: payload_off as u64,
        byte_length: (64 + if full_codec { 64 } else { 0 }) as u32,
        record_count: 1,
        srid_geodetic_code: 4326,
        lut_bank_epoch_id: 2,
        chunk_record_mode: M64_MODE_STRATA,
        compression_flags: if full_codec { M64_CODEC_BLAKE3_FULL } else { 2 },
        bao_root_hash_prefix: [0; 12],
        entry_crc32c: 0,
    };
    if !full_codec {
        row.bao_root_hash_prefix
            .copy_from_slice(&blake3::hash(record).as_bytes()[..12]);
    }
    row.update_crc();
    let mut out = vec![0; total];
    out[..64].copy_from_slice(bytemuck::bytes_of(&header));
    if with_metric {
        let mut metric = M64LocalMetricHeader::default();
        metric.anchor_did_hash = 0x1122_3344_5566_7788;
        metric.anisotropic_scale_um = [2.0, 3.0, 4.0];
        metric.update_crc();
        out[64..192].copy_from_slice(bytemuck::bytes_of(&metric));
    }
    out[dir..dir + 64].copy_from_slice(bytemuck::bytes_of(&row));
    out[payload_off..rec_end].copy_from_slice(record);
    if full_codec {
        out[rec_end..].copy_from_slice(&blake3_trailer(record));
    }
    out
}

// Each range is made only after an actual publisher/closure selection and a real
// production leaf-span verification over this exact fixture's source bytes.
macro_rules! with_qbdl_span {
    ($bytes:expr, |$selected:ident, $span:ident| $body:block) => {{
        let bytes = $bytes;
        let fixture = Fixture::new_with_payload(&bytes);
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let closure = fixture.bound(&pins, pin, publisher);
        let header_view = admit_header(
            &fixture.source()[..64],
            fixture.source().len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let mut entry_scratch = [EntryScratch::EMPTY; 1];
        let index = admit_index(
            &header_view,
            &fixture.source()[header_view.index_offset() as usize..],
            IndexReadLimits::default(),
            &mut entry_scratch,
        )
        .unwrap();
        let mut entry_rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        let $selected = select_entry(
            &closure,
            fixture.body(),
            &fixture.source()[..64],
            &index,
            fixture.entry_page(),
            0,
            &mut entry_rows,
            &mut framing,
        )
        .unwrap();
        let mut sibling_scratch = [StrongDigest::ZERO; 32];
        let $span = verify_leaf_span(
            &$selected.tree_spec().unwrap(),
            $selected.root(),
            0,
            &bytes,
            &[&[0]],
            &mut sibling_scratch,
            SpanLimits {
                max_leaves: 1,
                max_encoded_bytes: 16_384,
            },
        )
        .unwrap();
        $body
    }};
}
fn range<'a>(
    selected: crate::bundle::stream::selection::SelectedPlainEntry<'a>,
    span: &'a crate::bundle::stream::proof::VerifiedLeafSpan<'_>,
    offset: u64,
    len: usize,
) -> AuthenticatedRange<'a> {
    AuthenticatedRange::new(selected, span, offset, len).unwrap()
}
fn directory_offset(with_metric: bool) -> usize {
    if with_metric {
        192
    } else {
        64
    }
}
fn payload_offset(with_metric: bool) -> usize {
    if with_metric {
        256
    } else {
        128
    }
}

#[test]
fn m64_and_e64_full_digest_chunks_are_borrowed_and_zero_heap() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    for magic in [*b"m64\0", E64_MAGIC] {
        let bytes = m64_bytes(false, magic, true);
        with_qbdl_span!(&bytes, |selected, span| {
            let h = range(selected, &span, 0, 64);
            let d = range(selected, &span, 64, 64);
            let p = range(selected, &span, 128, 128);
            let mut scratch = [PayloadInterval::default(); 4];
            let guard = AllocGuard::begin("m64-e64-selective-full-digest", true);
            let view = M64DirectoryView::admit(h, d, None, m64_limits(), 8, &mut scratch).unwrap();
            let chunk = view.selected_chunk(0, p).unwrap();
            assert_eq!(
                chunk.records().as_ptr(),
                bytes[payload_offset(false)..].as_ptr()
            );
            assert_eq!(chunk.records().len(), 64);
            assert_eq!(chunk.entry().compression_flags, M64_CODEC_BLAKE3_FULL);
            assert_eq!(chunk.record(0).unwrap().len(), 64);
            assert_eq!(chunk.record(1), None);
            let decoded: M64StrataColumnRecord =
                bytemuck::pod_read_unaligned(chunk.record(0).unwrap());
            assert_eq!(decoded.surface_elevation_mm, 1234);
            assert!(decoded.validate().is_ok());
            assert_eq!(view.header().magic, magic);
            guard.check().unwrap();
        });
    }
}

#[test]
fn m64_full_blake3_trailer_checks_every_digest_and_reserved_byte() {
    let good = m64_bytes(false, *b"m64\0", true);
    for at in [192, 223, 224, 255] {
        let mut bad = good.clone();
        bad[at] ^= 1;
        with_qbdl_span!(&bad, |selected, span| {
            let h = range(selected, &span, 0, 64);
            let d = range(selected, &span, 64, 64);
            let p = range(selected, &span, 128, 128);
            let view = M64DirectoryView::admit(
                h,
                d,
                None,
                m64_limits(),
                8,
                &mut [PayloadInterval::default(); 4],
            )
            .unwrap();
            assert!(
                matches!(view.selected_chunk(0, p), Err(FormatError::Integrity)),
                "trailer byte {at}"
            );
        });
    }
}

#[test]
fn m64_record_crc_is_checked_after_valid_full_trailer_and_outer_proof() {
    let good = m64_bytes(false, *b"m64\0", true);
    let mut bad_record = good.clone();
    bad_record[128] ^= 1;
    // Repair the BLAKE3 trailer so the record-level CRC is the failing layer.
    let trailer = blake3_trailer(&bad_record[128..192]);
    bad_record[192..256].copy_from_slice(&trailer);
    with_qbdl_span!(&bad_record, |selected, span| {
        let h = range(selected, &span, 0, 64);
        let d = range(selected, &span, 64, 64);
        let p = range(selected, &span, 128, 128);
        let view = M64DirectoryView::admit(
            h,
            d,
            None,
            m64_limits(),
            8,
            &mut [PayloadInterval::default(); 4],
        )
        .unwrap();
        assert!(matches!(
            view.selected_chunk(0, p),
            Err(FormatError::Integrity)
        ));
    });
}

#[test]
fn m64_lrm3_is_an_authenticated_required_metadata_range() {
    let bytes = m64_bytes(true, *b"m64\0", true);
    with_qbdl_span!(&bytes, |selected, span| {
        let h = range(selected, &span, 0, 64);
        let metric = range(selected, &span, 64, 128);
        let d = range(selected, &span, 192, 64);
        let p = range(selected, &span, 256, 128);
        let view = M64DirectoryView::admit(
            h,
            d,
            Some(metric),
            m64_limits(),
            8,
            &mut [PayloadInterval::default(); 4],
        )
        .unwrap();
        assert_eq!(
            view.local_metric().unwrap().anchor_did_hash,
            0x1122_3344_5566_7788
        );
        let chunk = view.selected_chunk(0, p).unwrap();
        assert_eq!(chunk.records().len(), 64);
        let missing = M64DirectoryView::admit(
            h,
            d,
            None,
            m64_limits(),
            8,
            &mut [PayloadInterval::default(); 4],
        );
        assert!(matches!(missing, Err(FormatError::Bounds)));
        let shifted = range(selected, &span, 65, 128);
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                Some(shifted),
                m64_limits(),
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Bounds)
        ));
        let limited = FormatLimits {
            max_metadata_bytes: 255,
            ..m64_limits()
        };
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                Some(metric),
                limited,
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Limit)
        ));
    });
    let mut corrupt_metric = bytes.clone();
    corrupt_metric[64] ^= 1;
    with_qbdl_span!(&corrupt_metric, |selected, span| {
        let h = range(selected, &span, 0, 64);
        let metric = range(selected, &span, 64, 128);
        let d = range(selected, &span, 192, 64);
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                Some(metric),
                m64_limits(),
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Integrity)
        ));
    });
}

#[test]
fn m64_refuses_legacy_prefix_bad_extents_and_budget_violations() {
    let legacy = m64_bytes(false, *b"m64\0", false);
    with_qbdl_span!(&legacy, |selected, span| {
        let h = range(selected, &span, 0, 64);
        let d = range(selected, &span, 64, 64);
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                None,
                m64_limits(),
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Integrity)
        ));
    });
    let valid = m64_bytes(false, *b"m64\0", true);
    // Directory row is re-CRC'd and all bytes receive a fresh outer proof so the
    // following failures exercise format admission, not QBDL selection.
    for mutate in [0u8, 1] {
        let mut bad = valid.clone();
        let mut row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bad[64..128]);
        if mutate == 0 {
            row.byte_offset = 64;
        } else {
            row.byte_offset = u64::MAX - 63;
        }
        row.update_crc();
        bad[64..128].copy_from_slice(bytemuck::bytes_of(&row));
        with_qbdl_span!(&bad, |selected, span| {
            let h = range(selected, &span, 0, 64);
            let d = range(selected, &span, 64, 64);
            assert!(M64DirectoryView::admit(
                h,
                d,
                None,
                m64_limits(),
                8,
                &mut [PayloadInterval::default(); 4]
            )
            .is_err());
        });
    }
    with_qbdl_span!(&valid, |selected, span| {
        let h = range(selected, &span, 0, 64);
        let d = range(selected, &span, 64, 64);
        let p = range(selected, &span, 128, 128);
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                None,
                FormatLimits {
                    max_rows: 0,
                    ..m64_limits()
                },
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Limit)
        ));
        assert!(matches!(
            M64DirectoryView::admit(h, d, None, m64_limits(), 8, &mut []),
            Err(FormatError::Capacity)
        ));
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                None,
                m64_limits(),
                0,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Limit)
        ));
        assert!(matches!(
            M64DirectoryView::admit(
                h,
                d,
                None,
                FormatLimits {
                    max_metadata_bytes: 127,
                    ..m64_limits()
                },
                8,
                &mut [PayloadInterval::default(); 4]
            ),
            Err(FormatError::Limit)
        ));
        let view = M64DirectoryView::admit(
            h,
            d,
            None,
            m64_limits(),
            8,
            &mut [PayloadInterval::default(); 4],
        )
        .unwrap();
        let wrong = range(selected, &span, 129, 127);
        assert!(matches!(
            view.selected_chunk(0, wrong),
            Err(FormatError::Bounds)
        ));
        assert!(matches!(
            view.selected_chunk(usize::MAX, p),
            Err(FormatError::Bounds)
        ));
        assert!(matches!(
            view.selected_chunk(0, range(selected, &span, 128, 127)),
            Err(FormatError::Bounds)
        ));
        let tiny = FormatLimits {
            max_chunk_bytes: 127,
            ..m64_limits()
        };
        let view =
            M64DirectoryView::admit(h, d, None, tiny, 8, &mut [PayloadInterval::default(); 4])
                .unwrap();
        assert!(matches!(view.selected_chunk(0, p), Err(FormatError::Limit)));
    });
}
