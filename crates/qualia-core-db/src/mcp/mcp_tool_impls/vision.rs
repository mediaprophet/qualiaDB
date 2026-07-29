//! MCP tools for `specialized_libs::computer_vision` (MIG-V2 + product surface).

use super::{json_f64, json_str, json_u64, parse_tool_args, McpSystemError};
use serde_json::{json, Value};

/// Computer-vision ops over caller-supplied buffers (cold path JSON).
///
/// Ops: `list`, `rgb_to_gray`, `super_resolve`, `mesh_quality_cleanup`,
/// `class_score_to_sigma`, `capability_summary`.
#[cfg(not(target_arch = "wasm32"))]
pub fn computer_vision(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::computer_vision as cvlib;
    use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
    use crate::specialized_libs::computer_vision::spatial::{
        cleanup_mesh_ir, MeshCleanupOptions, MeshIR,
    };
    use crate::specialized_libs::computer_vision::sr::{
        super_resolve, ClassicalKernel, EnhancementMode, SrBackend, SrRequest,
    };

    let v = parse_tool_args(args)?;
    let op = json_str(&v, "op", "list");

    match op {
        "list" => Ok(json!({
            "library": "specialized_libs::computer_vision",
            "ops": [
                "list",
                "rgb_to_gray",
                "super_resolve",
                "mesh_quality_cleanup",
                "class_score_to_sigma",
                "capability_summary"
            ],
            "modules": ["cv", "ops", "sr", "bio", "embeddings", "gpu", "spatial"],
            "notes": "Pure algorithms; biosense consent stays in qualia-vision product crate."
        })
        .to_string()),
        "capability_summary" => Ok(json!({
            "classical_sr": ["nearest", "bilinear", "bicubic", "lanczos3"],
            "gpu": {
                "nearest": "Forge Resize2d on shared_gpu when available",
                "bicubic": "Forge Keys cubic WGSL on shared_gpu when available",
                "lanczos": "cpu_only"
            },
            "spatial": ["MeshIR", "export_obj", "export_stl", "quality_cleanup", "sigma_map"],
            "bio": ["histopathology", "radiomics", "dicom_lite", "tracking"],
        })
        .to_string()),
        "class_score_to_sigma" => {
            let class_hash = json_u64(&v, "class_hash", 0);
            let score = json_f64(&v, "score", 1.0) as f32;
            let sigma = cvlib::class_score_to_sigma(class_hash, score);
            Ok(json!({ "sigma": sigma, "class_hash": class_hash, "score": score }).to_string())
        }
        "rgb_to_gray" => {
            let width = json_u64(&v, "width", 0) as u32;
            let height = json_u64(&v, "height", 0) as u32;
            let rgb = json_u8_list(&v, "rgb")?;
            if width == 0 || height == 0 {
                return Err(McpSystemError::InvalidParameters);
            }
            let need = (width as usize) * (height as usize) * 3;
            if rgb.len() < need {
                return Err(McpSystemError::InvalidParameters);
            }
            let view = RgbView::new(width, height, width * 3, &rgb[..need])
                .ok_or(McpSystemError::InvalidParameters)?;
            let mut gray = vec![0u8; (width as usize) * (height as usize)];
            cvlib::rgb_to_gray_u8(view, &mut gray)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            // Cap response size: return stats + sample for large images.
            let sample: Vec<u8> = gray.iter().take(64).copied().collect();
            Ok(json!({
                "width": width,
                "height": height,
                "mean": gray.iter().map(|&x| x as f64).sum::<f64>() / gray.len().max(1) as f64,
                "sample_prefix": sample,
                "byte_len": gray.len(),
            })
            .to_string())
        }
        "super_resolve" => {
            let width = json_u64(&v, "width", 0) as u32;
            let height = json_u64(&v, "height", 0) as u32;
            let scale = json_u64(&v, "scale", 2) as u8;
            let kernel = json_str(&v, "kernel", "bicubic");
            let rgb = json_u8_list(&v, "rgb")?;
            if width == 0 || height == 0 || scale < 2 || scale > 4 {
                return Err(McpSystemError::InvalidParameters);
            }
            let need = (width as usize) * (height as usize) * 3;
            if rgb.len() < need {
                return Err(McpSystemError::InvalidParameters);
            }
            // Bound MCP payload: refuse huge frames (edge safety).
            if need > 512 * 512 * 3 {
                return Err(McpSystemError::InvalidParameters);
            }
            let ck = match kernel {
                "nearest" => ClassicalKernel::Nearest,
                "bilinear" => ClassicalKernel::Bilinear,
                "lanczos" | "lanczos3" => ClassicalKernel::Lanczos3,
                _ => ClassicalKernel::Bicubic,
            };
            let req = SrRequest {
                rgb: &rgb[..need],
                width,
                height,
                scale,
                backend: SrBackend::Classical(ck),
                mode: EnhancementMode::Sharpen,
            };
            let ow = width * scale as u32;
            let oh = height * scale as u32;
            let mut out = vec![0u8; (ow as usize) * (oh as usize) * 3];
            let report =
                super_resolve(&req, &mut out).map_err(|_| McpSystemError::InvalidParameters)?;
            let sample: Vec<u8> = out.iter().take(48).copied().collect();
            Ok(json!({
                "backend_id": report.backend_id,
                "device": report.device,
                "scale": report.scale,
                "out_width": report.out_width,
                "out_height": report.out_height,
                "generative": report.generative,
                "tile_count": report.tile_count,
                "sample_prefix": sample,
                "out_bytes": out.len(),
            })
            .to_string())
        }
        "mesh_quality_cleanup" => {
            let positions = v
                .get("positions")
                .and_then(Value::as_array)
                .ok_or(McpSystemError::InvalidParameters)?;
            let indices = v
                .get("indices")
                .and_then(Value::as_array)
                .ok_or(McpSystemError::InvalidParameters)?;
            let mut mesh = MeshIR::empty();
            for p in positions {
                let arr = p.as_array().ok_or(McpSystemError::InvalidParameters)?;
                if arr.len() < 3 {
                    return Err(McpSystemError::InvalidParameters);
                }
                mesh.positions.push([
                    arr[0].as_f64().unwrap_or(0.0) as f32,
                    arr[1].as_f64().unwrap_or(0.0) as f32,
                    arr[2].as_f64().unwrap_or(0.0) as f32,
                ]);
            }
            for i in indices {
                mesh.indices
                    .push(i.as_u64().ok_or(McpSystemError::InvalidParameters)? as u32);
            }
            mesh.recompute_bounds_and_hash();
            let weld = json_f64(&v, "weld_epsilon", 0.0) as f32;
            let rep = cleanup_mesh_ir(
                &mut mesh,
                MeshCleanupOptions {
                    weld_epsilon: weld,
                    min_area: 0.0,
                },
            )
            .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "vertices_in": rep.vertices_in,
                "vertices_out": rep.vertices_out,
                "triangles_in": rep.triangles_in,
                "triangles_out": rep.triangles_out,
                "degenerates_removed": rep.degenerates_removed,
                "vertices_welded": rep.vertices_welded,
                "content_hash": format!("0x{:016x}", mesh.content_hash),
            })
            .to_string())
        }
        _ => Err(McpSystemError::ToolNotFound),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn computer_vision(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotFound)
}

fn json_u8_list(v: &Value, key: &str) -> Result<Vec<u8>, McpSystemError> {
    let arr = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    arr.iter()
        .map(|x| {
            x.as_u64()
                .map(|n| n as u8)
                .ok_or(McpSystemError::InvalidParameters)
        })
        .collect()
}
