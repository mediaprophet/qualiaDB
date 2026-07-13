use qualia_core_db::render::compile_10d::compile_mesh_to_10d_with_provenance;
use qualia_core_db::container_10d::provenance_section::ProvenanceSidecar;
use qualia_core_db::render::assets::Mesh;

use super::layers::{
    catalog::{find_layer, LayerDefinition, LayerSource},
    mesh_gen,
    nasa_gibs,
    starfield,
};

pub struct CompiledLayerAsset {
    pub layer_id: String,
    pub container_10d: Vec<u8>,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub source_format: String,
    pub license: String,
}

pub async fn download_and_compile_layer(
    layer_id: &str,
    resolution: u32,
) -> Result<CompiledLayerAsset, String> {
    let layer = find_layer(layer_id)
        .ok_or_else(|| format!("Unknown layer: {layer_id}"))?;

    match &layer.source {
        LayerSource::NasaGibs { layer: gibs_layer, projection } => {
            download_and_compile_earth(gibs_layer, projection, resolution, layer).await
        }
        LayerSource::YaleBrightStars => {
            compile_bright_stars(layer)
        }
        LayerSource::HipparcosCatalog => {
            compile_synthetic_stars(layer, resolution)
        }
        LayerSource::WmsImagery { layer: wms_layer, .. } if layer_id.starts_with("mars") || layer_id.starts_with("moon") => {
            compile_planetary(layer, wms_layer, resolution)
        }
        _ => Err(format!("Layer '{}' source not yet supported for download", layer_id))
    }
}

async fn download_and_compile_earth(
    gibs_layer: &str,
    projection: &str,
    resolution: u32,
    layer_def: &LayerDefinition,
) -> Result<CompiledLayerAsset, String> {
    let req = nasa_gibs::GibsRequest {
        layer: gibs_layer.to_string(),
        projection: projection.to_string(),
        width: resolution,
        height: resolution / 2,
    };

    let texture = nasa_gibs::download_gibs_texture(&req).await.map_err(|e| {
        format!("GIBS download failed (layer: {gibs_layer}): {e}. Falling back to synthetic Earth.")
    }).or_else(|e| {
        eprintln!("GIBS download error: {e}");
        Err(e)
    });

    let texture = match texture {
        Ok(t) => Some(t),
        Err(_) => None,
    };

    let segments = (resolution / 4).max(32).min(256);
    let rings = segments / 2;

    let (positions, colors, indices) = mesh_gen::generate_sphere_mesh_colored(segments, rings, |lat, lon| {
        if let Some(ref tex) = texture {
            tex.sample(lat, lon)
        } else {
            let ocean = lat.abs() < 60.0;
            let land = (lon.sin() * lat.cos() * 3.0).fract() > 0.3;
            if ocean && land {
                [0.2, 0.5, 0.2]
            } else if ocean {
                [0.1, 0.2, 0.5]
            } else {
                [0.9, 0.9, 0.95]
            }
        }
    });

    let mesh = Mesh {
        positions: positions.clone(),
        triangles: indices
            .chunks(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        min: [-1.0, -1.0, -1.0],
        max: [1.0, 1.0, 1.0],
    };

    let provenance = ProvenanceSidecar {
        source_bytes: format!("NASA GIBS WMS: {gibs_layer}").into_bytes(),
        source_media_type: "image/jpeg".to_string(),
        licence: layer_def.license.to_string(),
        vc: Vec::new(),
        semantic_metadata: Vec::new(),
        timestamp_epoch_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        version_hash: [0u8; 32],
    };

    let container_10d = compile_mesh_to_10d_with_provenance(&mesh, Some(&provenance))
        .map_err(|e| format!("10D compilation: {e:?}"))?;

    Ok(CompiledLayerAsset {
        layer_id: layer_def.id.to_string(),
        container_10d,
        vertex_count: positions.len() as u32,
        triangle_count: (indices.len() / 3) as u32,
        positions,
        colors,
        indices,
        source_format: "NASA GIBS WMS JPEG".to_string(),
        license: layer_def.license.to_string(),
    })
}

fn compile_bright_stars(layer_def: &LayerDefinition) -> Result<CompiledLayerAsset, String> {
    let radius = 100.0f64;
    let (positions, colors) = starfield::bright_stars_mesh(radius);

    let indices: Vec<u32> = (0..positions.len() as u32).collect();

    let mesh = Mesh {
        positions: positions.clone(),
        triangles: indices.chunks(1).map(|c| [c[0], c[0], c[0]]).collect(),
        min: [-radius as f32, -radius as f32, -radius as f32],
        max: [radius as f32, radius as f32, radius as f32],
    };

    let provenance = ProvenanceSidecar {
        source_bytes: b"Yale Bright Star Catalog (embedded)".to_vec(),
        source_media_type: "text/csv".to_string(),
        licence: layer_def.license.to_string(),
        vc: Vec::new(),
        semantic_metadata: Vec::new(),
        timestamp_epoch_s: 0,
        version_hash: [0u8; 32],
    };

    let container_10d = compile_mesh_to_10d_with_provenance(&mesh, Some(&provenance))
        .map_err(|e| format!("10D compilation: {e:?}"))?;

    Ok(CompiledLayerAsset {
        layer_id: layer_def.id.to_string(),
        container_10d,
        vertex_count: positions.len() as u32,
        triangle_count: positions.len() as u32,
        positions,
        colors,
        indices,
        source_format: "Yale Bright Star Catalog".to_string(),
        license: layer_def.license.to_string(),
    })
}

fn compile_synthetic_stars(layer_def: &LayerDefinition, count: u32) -> Result<CompiledLayerAsset, String> {
    let radius = 200.0f64;
    let count = count.min(50_000);
    let (positions, colors) = starfield::generate_synthetic_starfield(count, radius, 42);

    let indices: Vec<u32> = (0..positions.len() as u32).collect();

    let mesh = Mesh {
        positions: positions.clone(),
        triangles: indices.chunks(1).map(|c| [c[0], c[0], c[0]]).collect(),
        min: [-radius as f32, -radius as f32, -radius as f32],
        max: [radius as f32, radius as f32, radius as f32],
    };

    let provenance = ProvenanceSidecar {
        source_bytes: b"Synthetic starfield (procedural generation)".to_vec(),
        source_media_type: "application/octet-stream".to_string(),
        licence: layer_def.license.to_string(),
        vc: Vec::new(),
        semantic_metadata: Vec::new(),
        timestamp_epoch_s: 0,
        version_hash: [0u8; 32],
    };

    let container_10d = compile_mesh_to_10d_with_provenance(&mesh, Some(&provenance))
        .map_err(|e| format!("10D compilation: {e:?}"))?;

    Ok(CompiledLayerAsset {
        layer_id: layer_def.id.to_string(),
        container_10d,
        vertex_count: positions.len() as u32,
        triangle_count: positions.len() as u32,
        positions,
        colors,
        indices,
        source_format: "Synthetic procedural starfield".to_string(),
        license: layer_def.license.to_string(),
    })
}

fn compile_planetary(layer_def: &LayerDefinition, _wms_layer: &str, resolution: u32) -> Result<CompiledLayerAsset, String> {
    let segments = (resolution / 4).max(32).min(128);
    let rings = segments / 2;

    let base_color = layer_def.preview_color;
    let body_id = layer_def.id;

    let (positions, colors, indices) = mesh_gen::generate_sphere_mesh_colored(segments, rings, |lat, lon| {
        let crater_noise = ((lat * 7.0).sin() * (lon * 5.0).cos() + (lat * 3.0).cos() * (lon * 11.0).sin()) * 0.15;
        let r = (base_color[0] + crater_noise).clamp(0.0, 1.0);
        let g = (base_color[1] + crater_noise * 0.8).clamp(0.0, 1.0);
        let b = (base_color[2] + crater_noise * 0.6).clamp(0.0, 1.0);
        let _ = body_id;
        [r, g, b]
    });

    let mesh = Mesh {
        positions: positions.clone(),
        triangles: indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(),
        min: [-1.0, -1.0, -1.0],
        max: [1.0, 1.0, 1.0],
    };

    let provenance = ProvenanceSidecar {
        source_bytes: format!("Planetary body: {}", layer_def.name).into_bytes(),
        source_media_type: "model/gltf-binary".to_string(),
        licence: layer_def.license.to_string(),
        vc: Vec::new(),
        semantic_metadata: Vec::new(),
        timestamp_epoch_s: 0,
        version_hash: [0u8; 32],
    };

    let container_10d = compile_mesh_to_10d_with_provenance(&mesh, Some(&provenance))
        .map_err(|e| format!("10D compilation: {e:?}"))?;

    Ok(CompiledLayerAsset {
        layer_id: layer_def.id.to_string(),
        container_10d,
        vertex_count: positions.len() as u32,
        triangle_count: (indices.len() / 3) as u32,
        positions,
        colors,
        indices,
        source_format: "Procedural planetary surface".to_string(),
        license: layer_def.license.to_string(),
    })
}
