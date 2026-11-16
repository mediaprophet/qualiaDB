use super::*;
#[wasm_bindgen]
impl QualiaPortal {
    pub fn set_gpu_pass_timing_enabled(&mut self, enabled: bool) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.set_gpu_pass_timing_enabled(enabled);
        }
    }
    /// Poll completed GPU timestamps without blocking. Call at diagnostic cadence, not per frame.
    pub fn gpu_pass_timings(&mut self) -> Result<JsValue, JsValue> {
        let Some(gpu) = self.gpu.as_mut() else {
            return js_sys::JSON::parse(r#"{"available":false,"reason":"No native GPU renderer"}"#);
        };
        let sample = gpu.gpu_pass_timing_snapshot();
        let (groups, source_instances, visible_instances, authored_triangles) =
            gpu.mesh_instance_group_counts();
        let workload = serde_json::json!({
            "source_geometry_vertices":self.body_vertex_count,
            "source_geometry_triangles":self.body_index_count / 3,
            "prototype_groups":groups,"source_instances":source_instances,
            "current_visible_instances":visible_instances,
            "authored_expanded_triangles":authored_triangles,
            "current_submitted_triangles":gpu.mesh_submitted_triangle_count(),
            "captured_at_observed_frame":sample.observed_frame,
            "scope":"Current camera-frustum-visible mesh index spans multiplied by their submitted instance counts. Excludes raster clipping, sky, water, projectors and additional depth/shadow passes. This is current submission state, not the older completed GPU timestamp sample."
        });
        let timings = |values: [f64; 6]| {
            serde_json::json!({
                "ao":values[0],"shadow":values[1],"picking":values[2],
                "forward_scene":values[3],"postprocess":values[4],"total":values[5]
            })
        };
        let present = sample.completed_frames > 0;
        let adapter = gpu.gpu_timing_adapter_info().map(|info| serde_json::json!({
            "name":info.name,"vendor_id":info.vendor,"device_id":info.device,
            "device_type":format!("{:?}",info.device_type),"backend":format!("{:?}",info.backend),
            "driver":info.driver,"driver_info":info.driver_info,
            "source":"Actual adapter used to create this portal device; privacy-redacted fields remain empty"
        }));
        let value = serde_json::json!({
            "available":sample.available,"enabled":sample.enabled,"selected_adapter":adapter,
            "mesh_workload":workload,
            "completed_frames":sample.completed_frames,"skipped_frames":sample.skipped_frames,
            "readback_errors":sample.readback_errors,"latest_frame":sample.latest_frame,
            "observed_frame":sample.observed_frame,"oldest_sample_frame":sample.oldest_sample_frame,
            "sample_age_frames":sample.observed_frame.saturating_sub(sample.latest_frame),
            "pass_ms":if present {timings(sample.latest_ms)} else {serde_json::Value::Null},
            "p50_ms":if present {timings(sample.p50_ms)} else {serde_json::Value::Null},
            "p95_ms":if present {timings(sample.p95_ms)} else {serde_json::Value::Null},
            "sample_count":sample.samples,"window_capacity":16,"readback_slots":4,
            "quantile_method":"nearest rank","gpu_duration_measured":present,
            "method":"Etalon GPU query timestamps at pass-range boundaries",
            "includes_timestamp_marker_overhead":true,
            "scope":"AO; cached cascaded shadows; queued picking; forward scene including sky/water; temporal/output postprocessing. Excludes CPU submission, presentation wait and timestamp readback.",
            "unavailable_reason":if sample.available {serde_json::Value::Null} else {serde_json::json!("Device timestamp queries or bounded telemetry reservation unavailable")}
        });
        js_sys::JSON::parse(&value.to_string())
    }
}
