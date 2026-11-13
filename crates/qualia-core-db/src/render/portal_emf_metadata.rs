//! Bounded browser access to field interpretation and original timed PCM tracks.
//! Metadata inspection is independent of display or listening conversion support.
use crate::q42::emf_container::EmfContainerReader;
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;

pub(super) fn inspect(bytes: &[u8]) -> Result<Object, JsValue> {
    let reader = EmfContainerReader::parse(bytes)
        .map_err(|error| JsValue::from_str(&format!("emf: {error}")))?;
    let result = Object::new();
    Reflect::set(&result, &"container_version".into(), &(reader.header().version as f64).into())?;
    Reflect::set(&result, &"integrity".into(), &"full-blake3-chunks-and-crc32c".into())?;
    let tracks = js_sys::Array::new();
    if let Some(extension) = reader.extension() {
        // Keep the exact JSON text: JavaScript Number cannot represent every i64
        // epoch/tick. Consumers needing exact clocks parse those integers losslessly.
        let metadata = serde_json::to_string(extension.metadata())
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        Reflect::set(&result, &"metadata_json".into(), &metadata.into())?;
        Reflect::set(&result, &"extension_integrity".into(), &"full-blake3".into())?;
        for (index, descriptor) in extension.metadata().audio_tracks.iter().enumerate() {
            let pcm = extension.audio_payload(index)
                .ok_or_else(|| JsValue::from_str("emf: verified audio payload missing"))?;
            let track = Object::new();
            Reflect::set(&track, &"id".into(), &descriptor.spec.id.as_str().into())?;
            Reflect::set(&track, &"pcm_f32_le".into(), &Uint8Array::from(pcm))?;
            tracks.push(&track);
        }
    } else {
        Reflect::set(&result, &"metadata_json".into(), &JsValue::NULL)?;
        Reflect::set(&result, &"extension_integrity".into(), &JsValue::NULL)?;
    }
    Reflect::set(&result, &"audio_tracks".into(), &tracks)?;
    Ok(result)
}
