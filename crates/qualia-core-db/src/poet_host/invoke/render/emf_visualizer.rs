//! `Render.emf_upload_field` / `Render.emf_render_slice` / `Render.emf_field_info`
//! invoke handlers — 5D EMF volumetric visualizer (plan §7.3 W4).
//!
//! These handlers expose the PortalGpu EMF pipeline to VibeScript, enabling
//! Studio to upload physics simulation field grids and render volumetric
//! slices with 10D manifold tags mapped to color.
//!
//! ## Invoke surface
//!
//! | ID | Arguments | Returns |
//! |----|-----------|---------|
//! | `Render.emf_upload_field` | `{ handle, cells: [{ amplitude, phase, frequency, scale, ... }], nx, ny, nz, nt, bounds: [f64] }` | `{ uploaded, cell_count }` |
//! | `Render.emf_render_slice` | `{ handle, slice_z, slice_t, bounds?, amplitude_scale?, phase_offset?, manifold_gain? }` | `{ rendered }` |
//! | `Render.emf_field_info` | `{ handle }` | `{ has_field, nx, ny, nz, nt, cell_count }` |

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::render::gpu::EmfFieldCell;

/// Parse a VibeScript record into an `EmfFieldCell`.
#[cfg(not(target_arch = "wasm32"))]
fn parse_cell(rec: &Value, span: Span, i: usize) -> Result<EmfFieldCell, Diagnostic> {
    let amplitude = args::rec_f64(rec, "amplitude").unwrap_or(0.0) as f32;
    let phase = args::rec_f64(rec, "phase").unwrap_or(0.0) as f32;
    let frequency = args::rec_f64(rec, "frequency").unwrap_or(0.0) as f32;
    let scale = args::rec_f64(rec, "scale").unwrap_or(0.0) as f32;
    let attention_depth = args::rec_f64(rec, "attention_depth").unwrap_or(0.0) as f32;
    let epistemic_weight = args::rec_f64(rec, "epistemic_weight").unwrap_or(0.0) as f32;
    let topological_spin = args::rec_f64(rec, "topological_spin").unwrap_or(0.0) as f32;
    let temporal_decay = args::rec_f64(rec, "temporal_decay").unwrap_or(0.0) as f32;
    let entropy_bias = args::rec_f64(rec, "entropy_bias").unwrap_or(0.0) as f32;
    let spatial_phase = args::rec_f64(rec, "spatial_phase").unwrap_or(0.0) as f32;
    let recurrence_frequency = args::rec_f64(rec, "recurrence_frequency").unwrap_or(0.0) as f32;
    let density_threshold = args::rec_f64(rec, "density_threshold").unwrap_or(0.0) as f32;
    let manifold_curvature = args::rec_f64(rec, "manifold_curvature").unwrap_or(0.0) as f32;
    let _ = (span, i);
    Ok(EmfFieldCell {
        amplitude,
        phase,
        frequency,
        scale,
        attention_depth,
        epistemic_weight,
        topological_spin,
        temporal_decay,
        entropy_bias,
        spatial_phase,
        recurrence_frequency,
        density_threshold,
        manifold_curvature,
    })
}

/// `Render.emf_upload_field` — upload an EMF field grid to the GPU.
pub fn emf_upload_field(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "emf_upload_field needs { handle: u64 }"))?;
        let nx = args::rec_u64(args, "nx").unwrap_or(0) as u32;
        let ny = args::rec_u64(args, "ny").unwrap_or(0) as u32;
        let nz = args::rec_u64(args, "nz").unwrap_or(0) as u32;
        let nt = args::rec_u64(args, "nt").unwrap_or(0) as u32;
        let cells_list = args::rec(args, "cells")
            .and_then(args::list)
            .ok_or_else(|| args::bad(span, "emf_upload_field needs { cells: [...] }"))?;
        let bounds = args::rec(args, "bounds")
            .and_then(args::list)
            .unwrap_or_default();
        let bounds_arr: [f32; 6] = if bounds.len() >= 6 {
            [
                bounds[0].as_f64().unwrap_or(-1.0) as f32,
                bounds[1].as_f64().unwrap_or(1.0) as f32,
                bounds[2].as_f64().unwrap_or(-1.0) as f32,
                bounds[3].as_f64().unwrap_or(1.0) as f32,
                bounds[4].as_f64().unwrap_or(-1.0) as f32,
                bounds[5].as_f64().unwrap_or(1.0) as f32,
            ]
        } else {
            [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0]
        };

        let mut cells = Vec::with_capacity(cells_list.len());
        for (i, cell_v) in cells_list.iter().enumerate() {
            cells.push(parse_cell(cell_v, span, i)?);
        }

        super::gpu::slot_with(handle, |portal| {
            portal.emf_upload_field(&cells, nx, ny, nz, nt, bounds_arr)
        })
        .ok_or_else(|| args::bad(span, "emf_upload_field: invalid handle"))?
        .map_err(|e| args::bad(span, format!("emf_upload_field: {e}")))?;

        Ok(args::record([
            ("uploaded", Value::Bool(true)),
            ("cell_count", Value::U64(cells.len() as u64)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "emf_upload_field requires native build with gpu-runtime",
        ))
    }
}

/// `Render.emf_render_slice` — render a 2D slice of the EMF field to the
/// portal's color target, then read back the rendered pixels.
///
/// This is a combined render + readback: it renders the slice and returns the
/// RGBA8 pixel data as a byte list, so VibeScript can display or save it.
pub fn emf_render_slice(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "emf_render_slice needs { handle: u64 }"))?;
        let slice_z = args::rec_u64(args, "slice_z").unwrap_or(0) as u32;
        let slice_t = args::rec_u64(args, "slice_t").unwrap_or(0) as u32;
        let amplitude_scale = args::rec_f64(args, "amplitude_scale").unwrap_or(1.0) as f32;
        let phase_offset = args::rec_f64(args, "phase_offset").unwrap_or(0.0) as f32;
        let manifold_gain = args::rec_f64(args, "manifold_gain").unwrap_or(1.0) as f32;
        let bounds = args::rec(args, "bounds")
            .and_then(args::list)
            .unwrap_or_default();
        let bounds_arr: [f32; 6] = if bounds.len() >= 6 {
            [
                bounds[0].as_f64().unwrap_or(-1.0) as f32,
                bounds[1].as_f64().unwrap_or(1.0) as f32,
                bounds[2].as_f64().unwrap_or(-1.0) as f32,
                bounds[3].as_f64().unwrap_or(1.0) as f32,
                bounds[4].as_f64().unwrap_or(-1.0) as f32,
                bounds[5].as_f64().unwrap_or(1.0) as f32,
            ]
        } else {
            [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0]
        };

        let (w, h, pixels) = super::gpu::slot_with(handle, |portal| {
            // Update slice params.
            portal.emf_update_slice_params(
                slice_z,
                slice_t,
                bounds_arr,
                amplitude_scale,
                phase_offset,
                manifold_gain,
            );
            // Render + readback in one call.
            portal.emf_render_slice_to_rgba8()
        })
        .ok_or_else(|| args::bad(span, "emf_render_slice: invalid handle"))?
        .map_err(|e| args::bad(span, format!("emf_render_slice: {e}")))?;

        // Return the pixel data as a u64 byte list.
        let pixel_values: Vec<Value> = pixels.iter().map(|&b| Value::U64(b as u64)).collect();
        Ok(args::record([
            ("rendered", Value::Bool(true)),
            ("width", Value::U64(w as u64)),
            ("height", Value::U64(h as u64)),
            ("pixels", Value::List(pixel_values)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "emf_render_slice requires native build with gpu-runtime",
        ))
    }
}

/// `Render.emf_field_info` — return metadata about the uploaded EMF field.
pub fn emf_field_info(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "emf_field_info needs { handle: u64 }"))?;
        super::gpu::slot_with(handle, |portal| {
            let (nx, ny, nz, nt) = portal.emf_grid_dims();
            args::record([
                ("has_field", Value::Bool(portal.emf_has_field())),
                ("nx", Value::U64(nx as u64)),
                ("ny", Value::U64(ny as u64)),
                ("nz", Value::U64(nz as u64)),
                ("nt", Value::U64(nt as u64)),
                (
                    "cell_count",
                    Value::U64((nx as u64) * (ny as u64) * (nz as u64) * (nt as u64)),
                ),
            ])
        })
        .ok_or_else(|| args::bad(span, "emf_field_info: invalid handle"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "emf_field_info requires native build with gpu-runtime",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{gpu_destroy, gpu_init};
    use super::*;

    fn snap() -> crate::poet_host::PoetSnapshot {
        crate::poet_host::PoetSnapshot::default()
    }

    #[allow(dead_code)]
    fn eval(src: &str) -> Value {
        let mut snap = snap();
        snap.eval_fn(src, "go", vec![]).expect("script should eval")
    }

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn g_emf_field_info_no_field() {
        if crate::gpu_context::try_shared_gpu().is_none() {
            eprintln!("[emf_invoke_test] no GPU adapter — skipping");
            return;
        }
        // Init a portal, then query field info (should report has_field=false).
        let init_args = args::record([
            ("width", Value::U64(16)),
            ("height", Value::U64(16)),
            ("particle_cap", Value::U64(64)),
        ]);
        let init_result = gpu_init(&init_args, dummy_span()).expect("init");
        let handle = args::rec_u64(&init_result, "handle").unwrap();

        let info_args = args::record([("handle", Value::U64(handle))]);
        let result = emf_field_info(&info_args, dummy_span()).expect("field_info");
        assert_eq!(args::rec(&result, "has_field"), Some(&Value::Bool(false)));
        assert_eq!(args::rec(&result, "cell_count"), Some(&Value::U64(0)));

        // Cleanup.
        let destroy_args = args::record([("handle", Value::U64(handle))]);
        let _ = gpu_destroy(&destroy_args, dummy_span());
    }

    #[test]
    fn g_emf_upload_and_render_direct() {
        if crate::gpu_context::try_shared_gpu().is_none() {
            eprintln!("[emf_invoke_test] no GPU adapter — skipping");
            return;
        }
        // Init portal.
        let init_args = args::record([
            ("width", Value::U64(16)),
            ("height", Value::U64(16)),
            ("particle_cap", Value::U64(64)),
        ]);
        let init_result = gpu_init(&init_args, dummy_span()).expect("init");
        let handle = args::rec_u64(&init_result, "handle").unwrap();

        // Upload a 2×2×1×1 field.
        let cells = vec![
            args::record([
                ("amplitude", Value::F64(1.0)),
                ("phase", Value::F64(0.0)),
                ("frequency", Value::F64(1.0e9)),
                ("scale", Value::F64(0.5)),
                ("manifold_curvature", Value::F64(0.1)),
            ]),
            args::record([
                ("amplitude", Value::F64(0.5)),
                ("phase", Value::F64(1.5708)),
                ("frequency", Value::F64(1.0e9)),
                ("scale", Value::F64(0.8)),
            ]),
            args::record([
                ("amplitude", Value::F64(0.3)),
                ("phase", Value::F64(3.14159)),
                ("frequency", Value::F64(2.0e9)),
            ]),
            args::record([
                ("amplitude", Value::F64(0.9)),
                ("phase", Value::F64(4.71239)),
                ("frequency", Value::F64(1.5e9)),
            ]),
        ];
        let upload_args = args::record([
            ("handle", Value::U64(handle)),
            ("cells", Value::List(cells)),
            ("nx", Value::U64(2)),
            ("ny", Value::U64(2)),
            ("nz", Value::U64(1)),
            ("nt", Value::U64(1)),
            (
                "bounds",
                Value::List(vec![
                    Value::F64(-1.0),
                    Value::F64(1.0),
                    Value::F64(-1.0),
                    Value::F64(1.0),
                    Value::F64(-1.0),
                    Value::F64(1.0),
                ]),
            ),
        ]);
        let upload_result = emf_upload_field(&upload_args, dummy_span()).expect("upload");
        assert_eq!(
            args::rec(&upload_result, "uploaded"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            args::rec(&upload_result, "cell_count"),
            Some(&Value::U64(4))
        );

        // Render slice.
        let render_args = args::record([
            ("handle", Value::U64(handle)),
            ("slice_z", Value::U64(0)),
            ("slice_t", Value::U64(0)),
            ("amplitude_scale", Value::F64(2.0)),
        ]);
        let render_result = emf_render_slice(&render_args, dummy_span()).expect("render");
        assert_eq!(
            args::rec(&render_result, "rendered"),
            Some(&Value::Bool(true))
        );
        let Value::List(pixels) = args::rec(&render_result, "pixels").unwrap() else {
            panic!("no pixels list")
        };
        assert_eq!(
            pixels.len(),
            16 * 16 * 4,
            "should have 16×16×4 RGBA8 pixels"
        );
        // At least some non-black pixels.
        let non_black = pixels
            .chunks(4)
            .filter(|px| {
                matches!(px[0], Value::U64(r) if r > 0)
                    || matches!(px[1], Value::U64(g) if g > 0)
                    || matches!(px[2], Value::U64(b) if b > 0)
            })
            .count();
        assert!(non_black > 0, "EMF slice should produce non-black pixels");

        // Cleanup.
        let destroy_args = args::record([("handle", Value::U64(handle))]);
        let _ = gpu_destroy(&destroy_args, dummy_span());
    }

    #[test]
    fn g_emf_field_info_via_vibescript() {
        // VibeScript's `.field` is namespace resolution, not record field
        // access, so we cannot chain `gpu_init` → `emf_field_info`. Instead,
        // verify the emf_field_info invoke is reachable from VibeScript and
        // returns an error for an invalid handle — confirming the dispatch
        // arm is wired.
        let src = r#"
        requires [ capability("capability.invoke") ];
        effect fn go() {
            return capability.invoke("Render.emf_field_info", { handle: 99999 });
        }
        "#;
        let mut snap = snap();
        let result = snap.eval_fn(src, "go", vec![]);
        assert!(
            result.is_err(),
            "expected error for invalid handle, got {result:?}"
        );
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid handle"), "got: {err:?}");
    }

    #[test]
    fn g_emf_render_without_field_errors() {
        if crate::gpu_context::try_shared_gpu().is_none() {
            eprintln!("[emf_invoke_test] no GPU adapter — skipping");
            return;
        }
        let init_args = args::record([
            ("width", Value::U64(16)),
            ("height", Value::U64(16)),
            ("particle_cap", Value::U64(64)),
        ]);
        let init_result = gpu_init(&init_args, dummy_span()).expect("init");
        let handle = args::rec_u64(&init_result, "handle").unwrap();

        let render_args = args::record([("handle", Value::U64(handle))]);
        let result = emf_render_slice(&render_args, dummy_span());
        assert!(result.is_err(), "render without field should error");

        let destroy_args = args::record([("handle", Value::U64(handle))]);
        let _ = gpu_destroy(&destroy_args, dummy_span());
    }
}
