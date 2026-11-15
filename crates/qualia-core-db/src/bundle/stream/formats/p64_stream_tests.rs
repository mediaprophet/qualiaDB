use super::super::AuthenticatedRange;
use super::{p64_present_id, P64AdmitError, P64Profile, P64StreamView};
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits, VerifiedLeafSpan},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::qdnf::types::StrongDigest;
use crate::p64_weight::{align_up, compile_gguf_to_p64, has_p64_magic, IntegrityMode, P64_MAGIC};
fn with_qbdl_span(payload: &[u8], run: impl FnOnce(SelectedPlainEntry<'_>, &VerifiedLeafSpan<'_>)) {
    let fixture = Fixture::new_with_payload(payload);
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
    let selected = select_entry(
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
    let span = verify_leaf_span(
        &selected.tree_spec().unwrap(),
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut sibling_scratch,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16_384,
        },
    )
    .unwrap();
    run(selected, &span);
}
fn complete<'a>(
    selected: SelectedPlainEntry<'a>,
    span: &'a VerifiedLeafSpan<'_>,
    payload: &[u8],
) -> AuthenticatedRange<'a> {
    AuthenticatedRange::new(selected, span, 0, payload.len()).unwrap()
}
fn admit_err(range: AuthenticatedRange<'_>) -> P64AdmitError {
    match P64StreamView::admit(range) {
        Err(e) => e,
        Ok(_) => panic!("admitted"),
    }
}
fn put_kv_u32(out: &mut Vec<u8>, key: &str, value: u32) {
    out.extend_from_slice(&(key.len() as u64).to_le_bytes());
    out.extend_from_slice(key.as_bytes());
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_tensor(out: &mut Vec<u8>, name: &str, dims: &[u64], offset: u64) {
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
    for dimension in dims {
        out.extend_from_slice(&dimension.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
}
fn synthetic_gguf() -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"GGUF");
    output.extend_from_slice(&3u32.to_le_bytes());
    output.extend_from_slice(&2u64.to_le_bytes());
    output.extend_from_slice(&3u64.to_le_bytes());
    put_kv_u32(&mut output, "llama.block_count", 1);
    put_kv_u32(&mut output, "llama.embedding_length", 4);
    put_kv_u32(&mut output, "llama.attention.head_count", 1);
    put_tensor(&mut output, "blk.0.attn_q.weight", &[4, 4], 0);
    put_tensor(&mut output, "token_embd.weight", &[4, 2], 64);
    output.resize(align_up(output.len(), 32), 0);
    for byte in 0u8..96 {
        output.push(byte.wrapping_mul(17).wrapping_add(3));
    }
    output
}
fn compiled_p64() -> Vec<u8> {
    compile_gguf_to_p64(&synthetic_gguf(), 8).expect("compile gguf to p64")
}
#[test]
fn complete_entry_admits_compiled_p64() {
    let payload = compiled_p64();
    assert!(has_p64_magic(&payload));
    assert_eq!(&payload[..4], &P64_MAGIC);
    with_qbdl_span(&payload, |selected, span| {
        let view = match P64StreamView::admit(complete(selected, span, &payload)) {
            Ok(v) => v,
            Err(_) => panic!("p64 complete-entry"),
        };
        assert_eq!(view.profile(), P64Profile::CompleteEntry);
        assert!(view.ready_for_plan());
        assert_eq!(p64_present_id(41, &view), Some(41));
        assert!(!view.index().entries.is_empty());
    });
}
#[test]
fn metadata_integrity_matches_default_admit() {
    let payload = compiled_p64();
    with_qbdl_span(&payload, |selected, span| {
        let range = complete(selected, span, &payload);
        let a = match P64StreamView::admit(range) {
            Ok(v) => v,
            Err(_) => panic!("default admit"),
        };
        let b = match P64StreamView::admit_with_integrity(range, IntegrityMode::Metadata) {
            Ok(v) => v,
            Err(_) => panic!("metadata admit"),
        };
        assert_eq!(a.index().entries.len(), b.index().entries.len());
    });
}
#[test]
fn blob_without_p64_magic_is_family() {
    let payload = b"qbdl-mixed-opaque-v1-not-p64";
    with_qbdl_span(payload, |selected, span| {
        match admit_err(complete(selected, span, payload)) {
            P64AdmitError::Family => {}
            _ => panic!("blob tagged as p64 must refuse Family"),
        }
    });
}
#[test]
fn gguf_magic_is_not_p64() {
    let payload = synthetic_gguf();
    with_qbdl_span(&payload, |selected, span| {
        match admit_err(complete(selected, span, &payload)) {
            P64AdmitError::Family => {}
            _ => panic!("GGUF is not a P64 complete-entry"),
        }
    });
}
#[test]
fn truncated_header_is_family_or_extent() {
    let payload = b"p64\0short";
    with_qbdl_span(payload, |selected, span| {
        match admit_err(complete(selected, span, payload)) {
            P64AdmitError::Family | P64AdmitError::Extent | P64AdmitError::Integrity => {}
            _ => panic!("truncated p64 header must refuse"),
        }
    });
}
#[test]
fn partial_range_is_extent() {
    let payload = compiled_p64();
    with_qbdl_span(&payload, |selected, span| {
        let range = AuthenticatedRange::new(selected, span, 0, payload.len() / 2).unwrap();
        match admit_err(range) {
            P64AdmitError::Extent => {}
            _ => panic!("partial p64 is Extent"),
        }
    });
}
