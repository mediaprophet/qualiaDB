use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Counting;
thread_local! { pub(super) static ALLOCS: Cell<Option<usize>> = const { Cell::new(None) }; }
fn allocation() {
    let _ = ALLOCS.try_with(|c| {
        if let Some(n) = c.get() {
            c.set(Some(n + 1));
        }
    });
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(p, layout, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

pub(super) fn budget() -> OpaquePreservationBudget {
    OpaquePreservationBudget {
        max_bytes: 4096,
        max_sections: 8,
    }
}
pub(super) fn fixture(version: u16) -> Vec<u8> {
    let mut header = Container10dHeader::proposed();
    header.version = version;
    let inputs = [
        SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 4,
            element_count: 2,
            payload: b"mesh0001",
        },
        SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0,
            element_count: 0,
            payload: b"field001",
        },
        SectionInput {
            section_type: SectionType::Materials,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: b"mat00001",
        },
    ];
    let mut out = vec![0; 1024];
    let len = encode_container(&header, &inputs, &mut out).unwrap();
    out.truncate(len);
    seal_whole_file_crc32c(&mut out);
    out
}
pub(super) fn descriptor_edit(
    data: &mut [u8],
    index: usize,
    edit: impl FnOnce(&mut SectionDescriptor),
) {
    let header = Container10dHeader::parse(data).unwrap();
    let start = header.section_table_offset as usize + index * SECTION_DESCRIPTOR_SIZE;
    let mut d = bytemuck::pod_read_unaligned::<SectionDescriptor>(&data[start..start + 24]);
    edit(&mut d);
    data[start..start + 24].copy_from_slice(bytemuck::bytes_of(&d));
    seal_whole_file_crc32c(data);
}
pub(super) fn header_edit(data: &mut [u8], edit: impl FnOnce(&mut Container10dHeader)) {
    let mut header = Container10dHeader::parse(data).unwrap();
    edit(&mut header);
    data[..64].copy_from_slice(&header.encode_to_vec64());
    seal_whole_file_crc32c(data);
}
pub(super) fn unknown_fixture() -> Vec<u8> {
    let mut data = fixture(2);
    descriptor_edit(&mut data, 2, |d| d.section_type = 230);
    data
}
pub(super) fn copy_error_keeps_output(data: &[u8]) -> OpaquePreservationError {
    let mut output = [0xA5; 1024];
    let error = copy_opaque_container_into(data, budget(), &mut output).unwrap_err();
    assert_eq!(output, [0xA5; 1024]);
    error
}
