//! Exact current-envelope metadata; structural inspection is independent of projection.
use crate::q42::field_envelope::{Family, Limits, StructuralView};
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;

pub(super) fn inspect(bytes: &[u8]) -> Result<Object, JsValue> {
    let view = StructuralView::open(bytes, Family::EmfV4Candidate, Limits::default())
        .map_err(|error| JsValue::from_str(&format!("emf envelope: {error:?}")))?;
    // The envelope bounds total bytes and entry count before any payload scan.
    for index in 1..=view.entry_count() {
        view.verify_chunk(index)
            .map_err(|error| JsValue::from_str(&format!("emf payload: {error:?}")))?;
    }
    let result = Object::new();
    Reflect::set(&result, &"container_version".into(), &4.0.into())?;
    Reflect::set(&result, &"integrity".into(), &"full-blake3-index-and-payloads-crc32c-header".into())?;
    Reflect::set(&result, &"admission".into(), &"structural; physical profile admission occurs before scientific use".into())?;
    // Preserve exact integer/rational/unknown fields without JavaScript Number conversion.
    Reflect::set(&result, &"metadata_tlv_le".into(), &Uint8Array::from(view.metadata_bytes()))?;
    Reflect::set(&result, &"source_blake3".into(), &blake3::hash(bytes).to_hex().as_str().into())?;
    Reflect::set(&result, &"entry_count".into(), &(view.entry_count() as f64).into())?;
    Ok(result)
}
