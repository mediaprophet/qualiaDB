//! Game-facing Qualia graphics controls and truthful execution receipts.
//!
//! The portal facade owns the WebGPU device lifecycle. This module owns the
//! settings that must survive asynchronous device adoption and recovery.

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

use crate::render::gpu::PortalGpu;
use crate::render::lighting::environment_lighting::{
    EnvironmentProbe, EnvironmentProbeSet, ENVIRONMENT_SPECULAR_LEVELS,
};

pub(crate) struct PortalGraphicsState {
    pub(crate) auto_temporal_enabled: bool,
    last_temporal_execution_observed: bool,
    temporal_execution_count: u32,
    shadows_enabled: bool,
    screen_space_ao_enabled: bool,
    screen_space_ao_params: [f32; 3],
    screen_space_ao_samples: u32,
    environment_probes: Option<EnvironmentProbeSet>,
    environment_fallback_radiance: [f32; 3],
    water_positions: Vec<[f32; 3]>,
    water_indices: Vec<u32>,
    water_surface: [f32; 6],
    water_quality: u32,
}

impl Default for PortalGraphicsState {
    fn default() -> Self {
        Self {
            auto_temporal_enabled: false,
            last_temporal_execution_observed: false,
            temporal_execution_count: 0,
            shadows_enabled: true,
            screen_space_ao_enabled: true,
            screen_space_ao_params: [0.45, 0.55, 0.025],
            screen_space_ao_samples: 8,
            environment_probes: None,
            environment_fallback_radiance: [0.18, 0.22, 0.26],
            water_positions: Vec::new(),
            water_indices: Vec::new(),
            water_surface: [0.16, 0.38, 0.42, 0.78, 0.025, 1.0],
            water_quality: 1,
        }
    }
}

impl PortalGraphicsState {
    pub(crate) fn set_auto_temporal_enabled(&mut self, enabled: bool) -> bool {
        if self.auto_temporal_enabled == enabled {
            return false;
        }
        self.auto_temporal_enabled = enabled;
        self.last_temporal_execution_observed = false;
        true
    }

    pub(crate) fn invalidate_temporal_history(&mut self, gpu: Option<&mut PortalGpu>) {
        self.last_temporal_execution_observed = false;
        if let Some(gpu) = gpu {
            gpu.invalidate_temporal_history();
        }
    }

    pub(crate) fn set_shadows_enabled(&mut self, enabled: bool, gpu: Option<&mut PortalGpu>) {
        self.shadows_enabled = enabled;
        if let Some(gpu) = gpu {
            gpu.set_shadows_enabled(enabled);
        }
    }

    pub(crate) fn shadows_available(gpu: Option<&PortalGpu>) -> bool {
        gpu.map(PortalGpu::shadows_enabled).unwrap_or(false)
    }

    pub(crate) fn set_screen_space_ao_enabled(
        &mut self,
        enabled: bool,
        gpu: Option<&mut PortalGpu>,
    ) {
        self.screen_space_ao_enabled = enabled;
        if let Some(gpu) = gpu {
            gpu.set_screen_space_ao_enabled(enabled);
        }
    }

    pub(crate) fn set_screen_space_ao(
        &mut self,
        radius: f32,
        strength: f32,
        bias: f32,
        gpu: Option<&mut PortalGpu>,
    ) {
        self.screen_space_ao_params = [radius, strength, bias];
        if let Some(gpu) = gpu {
            gpu.set_screen_space_ao(radius, strength, bias);
        }
    }

    pub(crate) fn set_screen_space_ao_sample_count(
        &mut self,
        samples: u32,
        gpu: Option<&mut PortalGpu>,
    ) {
        self.screen_space_ao_samples = samples;
        if let Some(gpu) = gpu {
            gpu.set_screen_space_ao_sample_count(samples);
        }
    }

    pub(crate) fn screen_space_ao_available(gpu: Option<&PortalGpu>) -> bool {
        gpu.map(PortalGpu::screen_space_ao_available)
            .unwrap_or(false)
    }

    pub(crate) fn set_environment_probe_data(
        &mut self,
        packed_probes: &[f32],
        fallback_radiance: &[f32],
        gpu: Option<&mut PortalGpu>,
    ) -> Result<bool, JsValue> {
        const RECORD_FLOATS: usize = 3 + 1 + 3 + ENVIRONMENT_SPECULAR_LEVELS * 3 + 3;
        if packed_probes.len() % RECORD_FLOATS != 0
            || packed_probes.len() / RECORD_FLOATS
                > crate::render::lighting::environment_lighting::MAX_ENVIRONMENT_PROBES
            || fallback_radiance.len() != 3
            || !packed_probes
                .iter()
                .chain(fallback_radiance)
                .all(|value| value.is_finite())
        {
            return Err(JsValue::from_str("invalid environment probe data"));
        }
        let mut probes = EnvironmentProbeSet::new();
        for record in packed_probes.chunks_exact(RECORD_FLOATS) {
            let mut specular_levels = [[0.0; 3]; ENVIRONMENT_SPECULAR_LEVELS];
            for (level, rgb) in specular_levels.iter_mut().enumerate() {
                let offset = 7 + level * 3;
                for axis in 0..3 {
                    rgb[axis] = record[offset + axis].max(0.0);
                }
            }
            let probe = EnvironmentProbe {
                position: [record[0], record[1], record[2]],
                radius: record[3].max(0.0),
                irradiance: [record[4].max(0.0), record[5].max(0.0), record[6].max(0.0)],
                specular_levels,
                dominant_direction: [record[22], record[23], record[24]],
                valid: record[3] > 0.0,
            };
            if !probes.push(probe) {
                return Err(JsValue::from_str("environment probe capacity exceeded"));
            }
        }
        self.environment_probes = if probes.is_empty() {
            None
        } else {
            Some(probes)
        };
        self.environment_fallback_radiance = [
            fallback_radiance[0].max(0.0),
            fallback_radiance[1].max(0.0),
            fallback_radiance[2].max(0.0),
        ];
        if let Some(gpu) = gpu {
            self.apply_environment_to_gpu(gpu);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn apply_environment_to_gpu(&self, gpu: &mut PortalGpu) {
        gpu.set_environment_probes(
            self.environment_probes.as_ref(),
            self.environment_fallback_radiance,
        );
    }

    pub(crate) fn set_water_geometry(
        &mut self,
        positions_xyz: &[f32],
        indices: &[u32],
        gpu: Option<&mut PortalGpu>,
    ) -> Result<bool, JsValue> {
        let vertex_count = positions_xyz.len() / 3;
        if positions_xyz.len() % 3 != 0
            || indices.len() % 3 != 0
            || vertex_count > crate::container_10d::MAX_VERTEX_COUNT as usize
            || indices.len() / 3 > crate::container_10d::MAX_TRIANGLE_COUNT as usize
            || !positions_xyz.iter().all(|value| value.is_finite())
            || indices.iter().any(|index| *index as usize >= vertex_count)
        {
            return Err(JsValue::from_str("invalid water mesh geometry"));
        }
        let positions: Vec<[f32; 3]> = positions_xyz
            .chunks_exact(3)
            .map(|point| [point[0], point[1], point[2]])
            .collect();
        if let Some(gpu) = gpu {
            if positions.is_empty() || indices.is_empty() {
                gpu.clear_water_geometry();
            } else {
                gpu.upload_water_geometry(&positions, indices)
                    .map_err(|error| JsValue::from_str(&error))?;
                self.apply_water_surface(gpu);
            }
            self.water_positions = positions;
            self.water_indices = indices.to_vec();
            Ok(true)
        } else {
            self.water_positions = positions;
            self.water_indices = indices.to_vec();
            Ok(false)
        }
    }

    pub(crate) fn set_water_surface(
        &mut self,
        surface: [f32; 6],
        quality: u32,
        gpu: Option<&mut PortalGpu>,
    ) {
        self.water_surface = surface;
        self.water_quality = quality;
        if let Some(gpu) = gpu {
            self.apply_water_surface(gpu);
        }
    }

    fn apply_water_surface(&self, gpu: &mut PortalGpu) {
        gpu.set_water_surface(
            [
                self.water_surface[0],
                self.water_surface[1],
                self.water_surface[2],
            ],
            self.water_surface[3],
            self.water_surface[4],
            self.water_surface[5],
        );
        gpu.set_water_quality(self.water_quality);
    }

    pub(crate) fn apply_to_gpu(&self, gpu: &mut PortalGpu) {
        gpu.set_shadows_enabled(self.shadows_enabled);
        gpu.set_screen_space_ao_enabled(self.screen_space_ao_enabled);
        gpu.set_screen_space_ao(
            self.screen_space_ao_params[0],
            self.screen_space_ao_params[1],
            self.screen_space_ao_params[2],
        );
        gpu.set_screen_space_ao_sample_count(self.screen_space_ao_samples);
        self.apply_environment_to_gpu(gpu);
        if !self.water_positions.is_empty() && !self.water_indices.is_empty() {
            let _ = gpu.upload_water_geometry(&self.water_positions, &self.water_indices);
            self.apply_water_surface(gpu);
        }
    }

    pub(crate) fn record_temporal_execution(&mut self, executed: bool) {
        self.last_temporal_execution_observed = executed;
        if executed {
            self.temporal_execution_count = self.temporal_execution_count.saturating_add(1);
        }
    }

    pub(crate) fn temporal_receipt(&self, gpu: Option<&PortalGpu>) -> JsValue {
        let (
            scene_color,
            linear_depth,
            motion_vectors,
            reactive_mask,
            history_valid,
            submission_state,
        ) = if let Some(gpu) = gpu {
            (
                gpu.temporal_scene_color_view().is_some(),
                gpu.temporal_resolve_gpu().is_some(),
                gpu.temporal_motion_view().is_some(),
                gpu.temporal_reactive_view().is_some(),
                gpu.temporal_history_valid(),
                format!("{:?}", gpu.temporal_submission_state()),
            )
        } else {
            (false, false, false, false, false, "idle".to_string())
        };
        let producer_ready = scene_color && linear_depth && motion_vectors && reactive_mask;
        let producers = Object::new();
        let receipt = Object::new();
        let _ = Reflect::set(&producers, &"scene_color".into(), &scene_color.into());
        let _ = Reflect::set(&producers, &"linear_depth".into(), &linear_depth.into());
        let _ = Reflect::set(&producers, &"motion_vectors".into(), &motion_vectors.into());
        let _ = Reflect::set(&producers, &"reactive_mask".into(), &reactive_mask.into());
        let _ = Reflect::set(&receipt, &"producer_views".into(), &producers.into());
        let _ = Reflect::set(&receipt, &"producer_ready".into(), &producer_ready.into());
        let _ = Reflect::set(
            &receipt,
            &"enabled".into(),
            &self.auto_temporal_enabled.into(),
        );
        let _ = Reflect::set(
            &receipt,
            &"execution_observed".into(),
            &self.last_temporal_execution_observed.into(),
        );
        let _ = Reflect::set(
            &receipt,
            &"execution_count".into(),
            &JsValue::from_f64(self.temporal_execution_count as f64),
        );
        let _ = Reflect::set(&receipt, &"history_valid".into(), &history_valid.into());
        let _ = Reflect::set(
            &receipt,
            &"submission_state".into(),
            &JsValue::from_str(&submission_state),
        );
        receipt.into()
    }
}
