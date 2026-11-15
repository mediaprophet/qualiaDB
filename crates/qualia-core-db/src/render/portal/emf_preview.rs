//! Explicit field draw; loading metadata or buffers alone is not a display proof.

use super::QualiaPortal;
use crate::render::portal_emf_state::EmfDisplayStatus;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Draw EMF slice z=0, t=0 on a WebGPU
    /// canvas no larger than 1024². The existing phase/sigma shader is a
    /// diagnostic derivative, not a calibrated RGB conversion. The receipt
    /// reports submission only; browser canvas readback is a separate check.
    /// The amplitude gain is a finite diagnostic display multiplier in (0, 1e6].
    /// It updates a shader uniform, never the retained physical cells.
    pub fn render_emf_preview(&mut self, amplitude_gain: f32) -> Result<JsValue, JsValue> {
        if !amplitude_gain.is_finite() || amplitude_gain <= 0.0 || amplitude_gain > 1.0e6 {
            return Err(JsValue::from_str(
                "EMF diagnostic amplitude gain must be finite and in (0, 1e6]",
            ));
        }
        self.note_device_loss();
        self.adopt_pending_gpu();
        if self.last_emf.status() != EmfDisplayStatus::Uploaded {
            return Err(JsValue::from_str(
                "EMF preview unavailable: field is not uploaded to WebGPU",
            ));
        }
        let bounds = self
            .last_emf
            .bounds()
            .ok_or_else(|| JsValue::from_str("EMF retained field unavailable"))?;
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("EMF preview requires WebGPU"))?;
        let (width, height) = gpu
            .emf_present_slice(bounds, amplitude_gain)
            .map_err(|error| JsValue::from_str(&error))?;
        let result = Object::new();
        Reflect::set(&result, &"width".into(), &JsValue::from_f64(width as f64))?;
        Reflect::set(&result, &"height".into(), &JsValue::from_f64(height as f64))?;
        Reflect::set(&result, &"backend".into(), &"webgpu".into())?;
        Reflect::set(&result, &"submitted".into(), &JsValue::TRUE)?;
        Reflect::set(&result, &"pixel_readback".into(), &"not-performed".into())?;
        Reflect::set(
            &result,
            &"conversion".into(),
            &"legacy-phase-sigma-diagnostic".into(),
        )?;
        Reflect::set(
            &result,
            &"amplitude_gain".into(),
            &JsValue::from_f64(amplitude_gain as f64),
        )?;
        Reflect::set(&result, &"z_slice".into(), &JsValue::from_f64(0.0))?;
        Reflect::set(&result, &"time_slice".into(), &JsValue::from_f64(0.0))?;
        Ok(result.into())
    }
}
