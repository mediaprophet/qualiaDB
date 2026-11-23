//! Explicit field draw; loading metadata or buffers alone is not a display proof.

use super::QualiaPortal;
use crate::render::portal_emf_state::EmfDisplayStatus;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Draw the current EMF slice on a portal GPU canvas no larger than 1024²,
    /// preserving the uploaded field's projection mode (`0` volumetric spectral +
    /// Stokes when optical carrier/sigma data is present, `1` electric-magnitude
    /// grayscale diagnostic when static). The receipt reports submission, the
    /// actual active backend, applied projection mode, and selected field/frame/time.
    pub fn render_emf_preview(&mut self, amplitude_gain: f32) -> Result<JsValue, JsValue> {
        self.render_emf_preview_internal(amplitude_gain, None)
    }

    /// Draw the current EMF slice with an explicit projection mode (`0` volumetric
    /// spectral + Stokes, `1` electric-magnitude grayscale diagnostic).
    pub fn render_emf_preview_with_mode(
        &mut self,
        amplitude_gain: f32,
        projection_mode: u32,
    ) -> Result<JsValue, JsValue> {
        if projection_mode > 1 {
            return Err(JsValue::from_str(
                "EMF preview projection_mode must be 0 (spectral) or 1 (grayscale diagnostic)",
            ));
        }
        self.render_emf_preview_internal(amplitude_gain, Some(projection_mode))
    }
}

impl QualiaPortal {
    fn render_emf_preview_internal(
        &mut self,
        amplitude_gain: f32,
        explicit_mode: Option<u32>,
    ) -> Result<JsValue, JsValue> {
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
        let field = self
            .last_emf
            .accepted()
            .ok_or_else(|| JsValue::from_str("EMF retained field unavailable"))?;
        let bounds = field.local_bounds();
        let selection = field.selection();
        let ticks = field.time_ticks().to_vec();
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("EMF preview requires WebGPU"))?;
        let applied_mode = explicit_mode.unwrap_or_else(|| gpu.emf_projection_mode());
        let (width, height) = gpu
            .emf_present_slice_with_mode(bounds, amplitude_gain, applied_mode)
            .map_err(|error| JsValue::from_str(&error))?;
        let (slice_z, slice_t) = gpu.emf_slice_indices();
        let backend = super::portal_gpu_backend_from_gpu(gpu).unwrap_or_else(|| "webgpu".to_string());
        let (conversion, display_formula) = if applied_mode == 0 {
            (
                "volumetric-spectral-stokes-projection",
                "full_spectrum_sigma_to_linear_rgb(sigma) * clamp(gain * (1-exp(-|E|/(1 V/m))), 0, 4) * (1 + 0.35 * DoP); missing transparent",
            )
        } else {
            (
                "electric-magnitude-grayscale-diagnostic",
                "clamp(gain * (1-exp(-|E|/(1 V/m))), 0, 1); validity alpha unchanged",
            )
        };
        let result = Object::new();
        Reflect::set(&result, &"width".into(), &JsValue::from_f64(width as f64))?;
        Reflect::set(&result, &"height".into(), &JsValue::from_f64(height as f64))?;
        Reflect::set(&result, &"backend".into(), &JsValue::from_str(&backend))?;
        Reflect::set(&result, &"submitted".into(), &JsValue::TRUE)?;
        Reflect::set(&result, &"pixel_readback".into(), &"not-performed".into())?;
        Reflect::set(
            &result,
            &"projection_mode".into(),
            &JsValue::from_f64(applied_mode as f64),
        )?;
        Reflect::set(&result, &"conversion".into(), &JsValue::from_str(conversion))?;
        Reflect::set(
            &result,
            &"display_formula".into(),
            &JsValue::from_str(display_formula),
        )?;
        Reflect::set(&result, &"source_values_changed".into(), &JsValue::FALSE)?;
        Reflect::set(
            &result,
            &"amplitude_gain".into(),
            &JsValue::from_f64(amplitude_gain as f64),
        )?;
        Reflect::set(&result, &"z_slice".into(), &JsValue::from_f64(slice_z as f64))?;
        Reflect::set(&result, &"time_slice".into(), &JsValue::from_f64(slice_t as f64))?;
        Reflect::set(
            &result,
            &"descriptor_id".into(),
            &JsValue::from_f64(selection.descriptor_id as f64),
        )?;
        Reflect::set(
            &result,
            &"frame_id".into(),
            &JsValue::from_f64(selection.frame_id as f64),
        )?;
        Reflect::set(
            &result,
            &"clock_id".into(),
            &JsValue::from_f64(selection.clock_id as f64),
        )?;
        Reflect::set(
            &result,
            &"stream_id".into(),
            &JsValue::from_f64(selection.stream_id as f64),
        )?;
        if let Some(&tick) = ticks.get(slice_t as usize) {
            Reflect::set(
                &result,
                &"time_tick_decimal".into(),
                &JsValue::from_str(&tick.to_string()),
            )?;
        }
        Ok(result.into())
    }
}
