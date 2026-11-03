use super::super::QTensorEngine;
use crate::specialized_libs::computational_geometry::allocation_counter::{
    assert_zero_alloc, AllocGuard,
};
use std::sync::Arc;

fn bytes(values: &[u8]) -> Arc<memmap2::Mmap> {
    let mut mapping = memmap2::MmapMut::map_anon(values.len()).unwrap();
    mapping.copy_from_slice(values);
    Arc::new(mapping.make_read_only().unwrap())
}

#[test]
fn borrowed_access_preserves_empty_field_and_exact_legacy_object() {
    let mut engine = QTensorEngine::new();
    assert!(engine.model_bytes().is_none());
    assert!(engine.clone_model_bytes().is_none());
    let original: Arc<memmap2::Mmap> = bytes(&[3, 5, 7, 11]);
    engine.gguf_mmap = Some(Arc::clone(&original));
    assert_eq!(engine.model_bytes().unwrap(), &[3, 5, 7, 11]);
    assert_eq!(engine.model_bytes().unwrap().as_ptr(), original.as_ptr());
    let retained = engine.clone_model_bytes().unwrap();
    assert_eq!(retained.as_ptr(), original.as_ptr());
    assert_eq!(retained.as_ref(), original.as_ref().as_ref());
    assert_eq!(Arc::strong_count(&original), 3);
    drop(retained);
    assert_eq!(Arc::strong_count(&original), 2);
}

#[test]
fn retained_clone_survives_field_replacement_and_engine_drop() {
    let mut engine = QTensorEngine::new();
    engine.gguf_mmap = Some(bytes(&[13, 17, 19]));
    let retained = engine.clone_model_bytes().unwrap();
    let pointer = retained.as_ptr();
    engine.gguf_mmap = Some(bytes(&[23, 29]));
    assert_eq!(engine.model_bytes().unwrap(), &[23, 29]);
    assert_eq!(&*retained, &[13, 17, 19]);
    let detached: Arc<memmap2::Mmap> = engine.gguf_mmap.take().unwrap();
    assert_eq!(&detached[..], &[23, 29]);
    assert!(engine.model_bytes().is_none());
    drop(engine);
    assert_eq!(retained.as_ptr(), pointer);
    assert_eq!(&*retained, &[13, 17, 19]);
    assert_eq!(&*retained.clone(), &[13, 17, 19]);
}

#[test]
fn actual_borrow_and_retained_clone_are_zero_alloc_with_positive_control() {
    let mut engine = QTensorEngine::new();
    let guard = AllocGuard::begin("model-byte positive allocation control", true);
    let allocated = std::hint::black_box(Box::new([31u8; 8]));
    assert_eq!(std::hint::black_box(allocated[0]), 31);
    drop(allocated);
    assert!(guard.check().is_err());
    assert_zero_alloc("absent model bytes", || {
        assert!(engine.model_bytes().is_none());
        assert!(engine.clone_model_bytes().is_none());
    });
    engine.gguf_mmap = Some(bytes(&[37, 41, 43, 47]));
    assert_zero_alloc("retained model-byte borrow and clone", || {
        for _ in 0..32 {
            let borrowed = engine.model_bytes().unwrap();
            let retained = engine.clone_model_bytes().unwrap();
            assert_eq!(borrowed.as_ptr(), retained.as_ptr());
            assert_eq!(retained.as_ref(), &[37, 41, 43, 47]);
            assert_eq!(retained.clone().as_ref(), borrowed);
        }
    });
}
