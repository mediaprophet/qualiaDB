//! Compile a model's organ meshes into the `.10d` body — the ingestion entry point (S5.4).
//!
//! Given a set of `(organ_key, mesh_bytes)` for an [`AnatomyModel`], resolve each organ's body system
//! and compile it to a sealed `.10d` whose q42 manifest carries `geo:bodySystem` + `geo:anatomyModel`.
//! Organs with no system mapping, or bytes that fail to parse, are **reported** — never silently
//! dropped. This layer owns the *compile*; the mesh **bytes** are supplied by the caller (the desktop
//! `glb_ingest` loads them from the CCF/HRA set chosen by `AnatomyModel::asset_set()`), so the file I/O
//! and the compile stay in their own lanes.

use qualia_core_db::render::compile_10d::{compile_organ_asset, CompiledAsset};
use wellfare_core::anatomy::{body_system_for_organ, AnatomyModel};

/// One organ compiled into the body: its resolved system and the sealed `.10d` asset.
pub struct CompiledOrgan {
    pub organ_key: String,
    pub system_id: String,
    pub asset: CompiledAsset,
}

/// The outcome of compiling a model's organ set — honest about what did and did not compile.
pub struct BodyCompileResult {
    pub model: AnatomyModel,
    pub organs: Vec<CompiledOrgan>,
    /// Organs with no body-system mapping (reported, not guessed onto a system).
    pub unmapped: Vec<String>,
    /// Organs whose bytes failed to import/compile, with the error text.
    pub failed: Vec<(String, String)>,
}

impl BodyCompileResult {
    /// How many organs compiled into the body.
    pub fn compiled_count(&self) -> usize {
        self.organs.len()
    }
}

/// The source-format hint for an organ key, from its extension (default `glb` — the CCF asset format).
fn format_of(organ_key: &str) -> &'static str {
    match organ_key.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "obj" => "obj",
        "stl" => "stl",
        "gltf" => "gltf",
        _ => "glb",
    }
}

/// Compile a model's organ meshes into its `.10d` body asset set.
pub fn compile_body(model: AnatomyModel, organs: &[(String, Vec<u8>)]) -> BodyCompileResult {
    let mut compiled = Vec::new();
    let mut unmapped = Vec::new();
    let mut failed = Vec::new();
    for (organ_key, bytes) in organs {
        let Some(system_id) = body_system_for_organ(organ_key) else {
            unmapped.push(organ_key.clone());
            continue;
        };
        let fmt = format_of(organ_key);
        let uri = format!("urn:qualia:anatomy:{}:{organ_key}", model.as_str());
        match compile_organ_asset(bytes, Some(fmt), &uri, fmt, Some(system_id), Some(model.as_str())) {
            Ok(asset) => compiled.push(CompiledOrgan {
                organ_key: organ_key.clone(),
                system_id: system_id.to_string(),
                asset,
            }),
            Err(e) => failed.push((organ_key.clone(), e.to_string())),
        }
    }
    BodyCompileResult { model, organs: compiled, unmapped, failed }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A single OBJ triangle standing in for an organ mesh (real assets are GLB; import_glb is proven
    // separately in render::assets — this exercises the ingestion orchestration, not GLB parsing).
    const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    #[test]
    fn compile_body_resolves_systems_binds_facts_and_reports_unmapped() {
        let organs = vec![
            ("3d-vh-m-lung.obj".to_string(), TRI_OBJ.to_vec()),
            ("3d-vh-m-blood-vasculature.obj".to_string(), TRI_OBJ.to_vec()),
            ("3d-vh-m-flux-capacitor.obj".to_string(), TRI_OBJ.to_vec()),
        ];
        let result = compile_body(AnatomyModel::Male, &organs);
        assert_eq!(result.compiled_count(), 2);
        assert_eq!(result.unmapped, vec!["3d-vh-m-flux-capacitor.obj".to_string()]);
        assert!(result.failed.is_empty());

        // Systems resolved from the organ keys.
        let lung = result.organs.iter().find(|o| o.organ_key.contains("lung")).unwrap();
        assert_eq!(lung.system_id, "respiratory");
        let vasc = result.organs.iter().find(|o| o.organ_key.contains("vasculature")).unwrap();
        assert_eq!(vasc.system_id, "circulatory");

        // Each compiled organ carries its system + model facts and a sealed, larger-than-header .10d.
        for organ in &result.organs {
            let vals: Vec<&str> = organ.asset.lexicon.values().map(String::as_str).collect();
            assert!(vals.contains(&organ.system_id.as_str()), "bodySystem fact present");
            assert!(vals.contains(&"male"), "anatomyModel fact present");
            assert!(organ.asset.container_10d.len() > 64, "sealed .10d container");
        }
    }

    /// Real-asset harness: compile an actual CCF/HRA organ GLB end-to-end. Point `QUALIA_TEST_GLB` at a
    /// `.glb` fetched from the HRA CDN (see `ccf_resolver`). Ignored by default (needs the file on disk).
    #[test]
    #[ignore = "requires a real GLB on disk via QUALIA_TEST_GLB"]
    fn compile_real_ccf_organ_end_to_end() {
        let path = std::env::var("QUALIA_TEST_GLB").expect("set QUALIA_TEST_GLB to a .glb path");
        let bytes = std::fs::read(&path).expect("read glb");
        let src_len = bytes.len();
        let filename = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let result = compile_body(AnatomyModel::Male, &[(filename.clone(), bytes)]);
        assert_eq!(
            result.compiled_count(),
            1,
            "unmapped={:?} failed={:?}",
            result.unmapped,
            result.failed
        );
        let organ = &result.organs[0];
        // The .10d round-trips back to a mesh.
        let mesh =
            qualia_core_db::render::compile_10d::decode_10d_mesh(&organ.asset.container_10d).unwrap();
        eprintln!(
            "REAL CCF ORGAN {filename} → system={} · {} verts / {} tris · GLB {src_len} B → .10d {} B ({:.2}x)",
            organ.system_id,
            mesh.vertex_count(),
            mesh.triangle_count(),
            organ.asset.container_10d.len(),
            src_len as f64 / organ.asset.container_10d.len() as f64,
        );
    }

    /// Whole-body harness: discover the male organ set from the HRA SPARQL endpoint, fetch each GLB
    /// from its CDN URL, and compile the entire body. Live network + heavy — ignored by default.
    #[test]
    #[ignore = "live network: discovers + fetches + compiles a full model (QUALIA_TEST_MODEL=male|female)"]
    fn compile_full_body_from_sparql() {
        use crate::wellfair::ccf_resolver::{
            discover_ref_organs, fetch_glb, organs_for_model, HRA_SPARQL_ENDPOINT,
        };
        let model = match std::env::var("QUALIA_TEST_MODEL").as_deref() {
            Ok("female") => AnatomyModel::Female,
            _ => AnatomyModel::Male,
        };
        let all = discover_ref_organs(HRA_SPARQL_ENDPOINT).expect("SPARQL discovery");
        let set = organs_for_model(&all, model);
        eprintln!("discovered {} total organs, {} {}", all.len(), set.len(), model.as_str());
        assert!(set.len() > 20, "expected a full {} set, got {}", model.as_str(), set.len());

        let mut fetched = Vec::new();
        let mut total_glb = 0usize;
        for organ in &set {
            match fetch_glb(&organ.glb_url) {
                Ok(bytes) => {
                    total_glb += bytes.len();
                    fetched.push((organ.filename.clone(), bytes));
                }
                Err(e) => eprintln!("  fetch FAILED {}: {e}", organ.filename),
            }
        }

        let result = compile_body(model, &fetched);
        let total_10d: usize = result.organs.iter().map(|o| o.asset.container_10d.len()).sum();
        let mut systems: Vec<&str> = result.organs.iter().map(|o| o.system_id.as_str()).collect();
        systems.sort();
        systems.dedup();
        eprintln!(
            "{} BODY: {} / {} organs compiled · {} systems {:?} · unmapped={:?} failed={:?} · GLB {} B → .10d {} B ({:.2}x)",
            model.as_str().to_uppercase(),
            result.compiled_count(),
            set.len(),
            systems.len(),
            systems,
            result.unmapped,
            result.failed.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            total_glb,
            total_10d,
            total_glb as f64 / total_10d.max(1) as f64,
        );
        // Every fetched male organ must resolve to a system — the map covers the real full set.
        assert!(result.unmapped.is_empty(), "unmapped organs: {:?}", result.unmapped);
        assert!(result.failed.is_empty(), "failed organs: {:?}", result.failed);
    }

    #[test]
    fn bad_bytes_are_reported_not_silently_dropped() {
        // A key that resolves to a system (lung → respiratory) but whose bytes are not a valid mesh.
        let organs = vec![("3d-vh-f-lung.glb".to_string(), vec![0u8, 1, 2, 3])];
        let result = compile_body(AnatomyModel::Female, &organs);
        assert_eq!(result.compiled_count(), 0);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].0, "3d-vh-f-lung.glb");
    }
}
