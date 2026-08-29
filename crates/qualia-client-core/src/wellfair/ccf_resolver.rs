//! Discover the CCF/HRA reference-organ GLB assets from the Human Reference Atlas **SPARQL endpoint**
//! (the scalable, canonical source — no repo clone, no Git-LFS pointers). The HRA Knowledge Graph
//! registers each reference organ as linked data; the GLB lives as a `foaf:depiction` `xsd:anyURI`
//! pointing at `cdn.humanatlas.io` (the real binary). The CDN path encodes organ, sex, and version, so
//! the discovered filename (e.g. `3d-vh-m-liver.glb`) feeds straight into
//! [`body_system_for_organ`](wellfare_core::anatomy::body_system_for_organ) and
//! [`compile_body`](super::anatomy_body::compile_body).
//!
//! This module is **pure** (query construction + result parsing + model filtering) — no HTTP, so it is
//! unit-tested against captured real endpoint JSON. The live GET + the per-organ binary fetch are a thin
//! transport layer (qualia-client-core's async HTTP is a separate lane); this owns the semantics.

use serde::{Deserialize, Serialize};
use wellfare_core::anatomy::AnatomyModel;

/// The HRA Linked Open Data SPARQL endpoint (verified live: returns `application/sparql-results+json`).
pub const HRA_SPARQL_ENDPOINT: &str = "https://lod.humanatlas.io/sparql";

/// A descriptive User-Agent. Some asset hosts (e.g. NIH 3D's WAF) reject requests with **no**
/// User-Agent (reqwest sends none by default) with a 403 — an explicit one is required.
const HTTP_USER_AGENT: &str = "QualiaDB-anatomy/1.0";

/// The query that lists every reference-organ GLB URL registered in the HRA KG (across named graphs).
/// Bound variable is `glb`. Deterministic ordering so the manifest is stable/attestable.
pub fn ref_organ_glb_query() -> String {
    "SELECT DISTINCT ?glb WHERE { \
       GRAPH ?g { ?s <http://xmlns.com/foaf/0.1/depiction> ?glb } \
       FILTER(STRENDS(LCASE(STR(?glb)), \".glb\") && CONTAINS(STR(?glb), \"/ref-organ/\")) \
     } ORDER BY ?glb"
        .to_string()
}

/// One discovered reference-organ asset: the GLB filename (the organ key used everywhere downstream),
/// its canonical CDN URL, and which reference model it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefOrgan {
    /// The GLB filename, e.g. `3d-vh-m-liver.glb` — this is the `organ_key` for `body_system_for_organ`.
    pub filename: String,
    /// The canonical CDN URL of the binary GLB.
    pub glb_url: String,
    /// The reference model this organ belongs to (from the `-f-`/`-m-` sex marker in the filename).
    pub model: AnatomyModel,
}

/// Parse the SPARQL-results JSON from [`ref_organ_glb_query`] into the reference-organ manifest.
///
/// The sex/model is read from the unambiguous `-f-`/`-m-` filename infix (provider varies — `vh`,
/// `allen`, `sbu`, `nih` — but the sex marker does not). A GLB with no sex infix (rare; unsexed asset)
/// is skipped rather than guessed. Malformed JSON yields an empty manifest.
pub fn parse_ref_organs(sparql_json: &str) -> Vec<RefOrgan> {
    let root: serde_json::Value = match serde_json::from_str(sparql_json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let bindings = root
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array());
    let mut out = Vec::new();
    if let Some(bindings) = bindings {
        for b in bindings {
            let Some(url) = b
                .get("glb")
                .and_then(|g| g.get("value"))
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let filename = url.rsplit('/').next().unwrap_or(url).to_string();
            let Some(model) = model_from_filename(&filename) else {
                continue;
            };
            out.push(RefOrgan {
                filename,
                glb_url: url.to_string(),
                model,
            });
        }
    }
    out
}

/// The reference model a CCF GLB filename belongs to, from its `-f-`/`-m-` sex infix.
fn model_from_filename(filename: &str) -> Option<AnatomyModel> {
    if filename.contains("-f-") {
        Some(AnatomyModel::Female)
    } else if filename.contains("-m-") {
        Some(AnatomyModel::Male)
    } else {
        None
    }
}

/// The organs of a single model, in discovery order.
pub fn organs_for_model(organs: &[RefOrgan], model: AnatomyModel) -> Vec<RefOrgan> {
    organs
        .iter()
        .filter(|o| o.model == model)
        .cloned()
        .collect()
}

/// A live CCF discovery / fetch error.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub enum CcfError {
    Http(reqwest::Error),
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for CcfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CcfError::Http(e) => write!(f, "CCF HTTP: {e}"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::error::Error for CcfError {}

#[cfg(not(target_arch = "wasm32"))]
impl From<reqwest::Error> for CcfError {
    fn from(e: reqwest::Error) -> Self {
        CcfError::Http(e)
    }
}

/// Discover the reference-organ manifest **live** from the HRA SPARQL endpoint (blocking network I/O —
/// call off the async runtime, e.g. via `spawn_blocking`). The query/parse are pure and unit-tested;
/// this only adds the transport. If the live endpoint returns an HTTP error (e.g. 502 Bad Gateway)
/// or times out, it gracefully falls back to the embedded CCF reference-organ manifest.
#[cfg(not(target_arch = "wasm32"))]
pub fn discover_ref_organs(endpoint: &str) -> Result<Vec<RefOrgan>, CcfError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .unwrap_or_default();

    let res = client
        .post(endpoint)
        .header(reqwest::header::USER_AGENT, HTTP_USER_AGENT)
        .header(reqwest::header::CONTENT_TYPE, "application/sparql-query")
        .header(reqwest::header::ACCEPT, "application/sparql-results+json")
        .body(ref_organ_glb_query())
        .send();

    match res {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(text) = resp.text() {
                let organs = parse_ref_organs(&text);
                if !organs.is_empty() {
                    return Ok(organs);
                }
            }
        }
        Ok(resp) => {
            eprintln!(
                "Warning: HRA SPARQL endpoint '{endpoint}' returned status {}. Falling back to embedded CCF manifest.",
                resp.status()
            );
            return Ok(fallback_ref_organs());
        }
        Err(e) => {
            eprintln!(
                "Warning: HRA SPARQL endpoint '{endpoint}' unreachable ({e}). Falling back to embedded CCF manifest."
            );
            return Ok(fallback_ref_organs());
        }
    }
    Ok(fallback_ref_organs())
}

/// Embedded fallback list of canonical CCF/HRA reference organ GLBs on `cdn.humanatlas.io`.
/// Used automatically when the live SPARQL endpoint is temporarily down (e.g. 502 Bad Gateway).
pub fn fallback_ref_organs() -> Vec<RefOrgan> {
    const FALLBACK_MANIFEST: &[(&str, &str, AnatomyModel)] = &[
        // Male organs
        ("3d-vh-m-brain.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/brain-male/v1.2/assets/3d-vh-m-brain.glb", AnatomyModel::Male),
        ("3d-vh-m-heart.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/heart-male/v1.2/assets/3d-vh-m-heart.glb", AnatomyModel::Male),
        ("3d-vh-m-lung-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/lung-male-left/v1.2/assets/3d-vh-m-lung-l.glb", AnatomyModel::Male),
        ("3d-vh-m-lung-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/lung-male-right/v1.2/assets/3d-vh-m-lung-r.glb", AnatomyModel::Male),
        ("3d-vh-m-liver.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/liver-male/v1.2/assets/3d-vh-m-liver.glb", AnatomyModel::Male),
        ("3d-vh-m-kidney-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/kidney-male-left/v1.3/assets/3d-vh-m-kidney-l.glb", AnatomyModel::Male),
        ("3d-vh-m-kidney-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/kidney-male-right/v1.3/assets/3d-vh-m-kidney-r.glb", AnatomyModel::Male),
        ("3d-vh-m-spleen.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/spleen-male/v1.2/assets/3d-vh-m-spleen.glb", AnatomyModel::Male),
        ("3d-vh-m-pancreas.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/pancreas-male/v1.2/assets/3d-vh-m-pancreas.glb", AnatomyModel::Male),
        ("3d-vh-m-small-intestine.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/small-intestine-male/v1.2/assets/3d-vh-m-small-intestine.glb", AnatomyModel::Male),
        ("3d-vh-m-large-intestine.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/large-intestine-male/v1.2/assets/3d-vh-m-large-intestine.glb", AnatomyModel::Male),
        ("3d-vh-m-urinary-bladder.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/urinary-bladder-male/v1.2/assets/3d-vh-m-urinary-bladder.glb", AnatomyModel::Male),
        ("3d-vh-m-prostate.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/prostate-male/v1.2/assets/3d-vh-m-prostate.glb", AnatomyModel::Male),
        ("3d-vh-m-trachea.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/trachea-male/v1.2/assets/3d-vh-m-trachea.glb", AnatomyModel::Male),
        ("3d-vh-m-larynx.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/larynx-male/v1.2/assets/3d-vh-m-larynx.glb", AnatomyModel::Male),
        ("3d-vh-m-main-bronchus.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/main-bronchus-male/v1.2/assets/3d-vh-m-main-bronchus.glb", AnatomyModel::Male),
        ("3d-vh-m-thymus.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/thymus-male/v1.2/assets/3d-vh-m-thymus.glb", AnatomyModel::Male),
        ("3d-vh-m-eye-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/eye-male-left/v1.2/assets/3d-vh-m-eye-l.glb", AnatomyModel::Male),
        ("3d-vh-m-eye-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/eye-male-right/v1.2/assets/3d-vh-m-eye-r.glb", AnatomyModel::Male),
        ("3d-vh-m-pelvis.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/pelvis-male/v1.2/assets/3d-vh-m-pelvis.glb", AnatomyModel::Male),
        ("3d-vh-m-skin.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/skin-male/v1.2/assets/3d-vh-m-skin.glb", AnatomyModel::Male),
        ("3d-vh-m-blood-vasculature.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/blood-vasculature-male/v1.2/assets/3d-vh-m-blood-vasculature.glb", AnatomyModel::Male),

        // Female organs
        ("3d-vh-f-brain.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/brain-female/v1.2/assets/3d-vh-f-brain.glb", AnatomyModel::Female),
        ("3d-vh-f-heart.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/heart-female/v1.2/assets/3d-vh-f-heart.glb", AnatomyModel::Female),
        ("3d-vh-f-lung-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/lung-female-left/v1.2/assets/3d-vh-f-lung-l.glb", AnatomyModel::Female),
        ("3d-vh-f-lung-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/lung-female-right/v1.2/assets/3d-vh-f-lung-r.glb", AnatomyModel::Female),
        ("3d-vh-f-liver.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/liver-female/v1.2/assets/3d-vh-f-liver.glb", AnatomyModel::Female),
        ("3d-vh-f-kidney-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/kidney-female-left/v1.3/assets/3d-vh-f-kidney-l.glb", AnatomyModel::Female),
        ("3d-vh-f-kidney-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/kidney-female-right/v1.3/assets/3d-vh-f-kidney-r.glb", AnatomyModel::Female),
        ("3d-vh-f-spleen.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/spleen-female/v1.2/assets/3d-vh-f-spleen.glb", AnatomyModel::Female),
        ("3d-vh-f-pancreas.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/pancreas-female/v1.2/assets/3d-vh-f-pancreas.glb", AnatomyModel::Female),
        ("3d-vh-f-small-intestine.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/small-intestine-female/v1.2/assets/3d-vh-f-small-intestine.glb", AnatomyModel::Female),
        ("3d-vh-f-large-intestine.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/large-intestine-female/v1.2/assets/3d-vh-f-large-intestine.glb", AnatomyModel::Female),
        ("3d-vh-f-urinary-bladder.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/urinary-bladder-female/v1.2/assets/3d-vh-f-urinary-bladder.glb", AnatomyModel::Female),
        ("3d-vh-f-uterus.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/uterus-female/v1.2/assets/3d-vh-f-uterus.glb", AnatomyModel::Female),
        ("3d-vh-f-ovary-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/ovary-female-left/v1.2/assets/3d-vh-f-ovary-l.glb", AnatomyModel::Female),
        ("3d-vh-f-ovary-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/ovary-female-right/v1.2/assets/3d-vh-f-ovary-r.glb", AnatomyModel::Female),
        ("3d-vh-f-fallopian-tube-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/fallopian-tube-female-left/v1.2/assets/3d-vh-f-fallopian-tube-l.glb", AnatomyModel::Female),
        ("3d-vh-f-fallopian-tube-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/fallopian-tube-female-right/v1.2/assets/3d-vh-f-fallopian-tube-r.glb", AnatomyModel::Female),
        ("3d-vh-f-vagina.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/vagina-female/v1.2/assets/3d-vh-f-vagina.glb", AnatomyModel::Female),
        ("3d-vh-f-trachea.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/trachea-female/v1.2/assets/3d-vh-f-trachea.glb", AnatomyModel::Female),
        ("3d-vh-f-larynx.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/larynx-female/v1.2/assets/3d-vh-f-larynx.glb", AnatomyModel::Female),
        ("3d-vh-f-main-bronchus.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/main-bronchus-female/v1.2/assets/3d-vh-f-main-bronchus.glb", AnatomyModel::Female),
        ("3d-vh-f-thymus.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/thymus-female/v1.2/assets/3d-vh-f-thymus.glb", AnatomyModel::Female),
        ("3d-vh-f-eye-l.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/eye-female-left/v1.2/assets/3d-vh-f-eye-l.glb", AnatomyModel::Female),
        ("3d-vh-f-eye-r.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/eye-female-right/v1.2/assets/3d-vh-f-eye-r.glb", AnatomyModel::Female),
        ("3d-vh-f-pelvis.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/pelvis-female/v1.2/assets/3d-vh-f-pelvis.glb", AnatomyModel::Female),
        ("3d-vh-f-skin.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/skin-female/v1.2/assets/3d-vh-f-skin.glb", AnatomyModel::Female),
        ("3d-vh-f-blood-vasculature.glb", "https://cdn.humanatlas.io/digital-objects/ref-organ/blood-vasculature-female/v1.2/assets/3d-vh-f-blood-vasculature.glb", AnatomyModel::Female),
    ];

    FALLBACK_MANIFEST
        .iter()
        .map(|(filename, url, model)| RefOrgan {
            filename: filename.to_string(),
            glb_url: url.to_string(),
            model: *model,
        })
        .collect()
}

/// Fetch one organ's GLB bytes from its CDN URL (blocking).
#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_glb(glb_url: &str) -> Result<Vec<u8>, CcfError> {
    let bytes = reqwest::blocking::Client::new()
        .get(glb_url)
        .header(reqwest::header::USER_AGENT, HTTP_USER_AGENT)
        .send()?
        .error_for_status()?
        .bytes()?;
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A captured, real fragment of the HRA endpoint response (three bindings — a male organ, a
    // lateralized female organ, and a female-only organ).
    const SAMPLE_JSON: &str = r#"{
      "head": { "vars": ["glb"] },
      "results": { "bindings": [
        { "glb": { "datatype": "http://www.w3.org/2001/XMLSchema#anyURI", "type": "literal",
          "value": "https://cdn.humanatlas.io/digital-objects/ref-organ/liver-male/v1.2/assets/3d-vh-m-liver.glb" } },
        { "glb": { "datatype": "http://www.w3.org/2001/XMLSchema#anyURI", "type": "literal",
          "value": "https://cdn.humanatlas.io/digital-objects/ref-organ/kidney-female-left/v1.3/assets/3d-vh-f-kidney-l.glb" } },
        { "glb": { "datatype": "http://www.w3.org/2001/XMLSchema#anyURI", "type": "literal",
          "value": "https://cdn.humanatlas.io/digital-objects/ref-organ/uterus-female/v1.2/assets/3d-vh-f-uterus.glb" } }
      ]}
    }"#;

    #[test]
    fn query_is_stable_and_targets_ref_organ_depictions() {
        let q = ref_organ_glb_query();
        assert!(q.contains("foaf/0.1/depiction"));
        assert!(q.contains("/ref-organ/"));
        assert!(q.contains("ORDER BY ?glb"), "deterministic manifest");
    }

    #[test]
    fn parses_real_endpoint_json_into_typed_manifest() {
        let organs = parse_ref_organs(SAMPLE_JSON);
        assert_eq!(organs.len(), 3);
        // Filename is the downstream organ key; the CDN URL is the real binary.
        assert_eq!(organs[0].filename, "3d-vh-m-liver.glb");
        assert!(organs[0].glb_url.starts_with("https://cdn.humanatlas.io/"));
        // Sex/model read from the -m-/-f- infix.
        assert_eq!(organs[0].model, AnatomyModel::Male);
        assert_eq!(organs[1].model, AnatomyModel::Female);
        assert_eq!(organs[2].model, AnatomyModel::Female);
    }

    #[test]
    fn model_filter_splits_the_body() {
        let organs = parse_ref_organs(SAMPLE_JSON);
        assert_eq!(organs_for_model(&organs, AnatomyModel::Male).len(), 1);
        assert_eq!(organs_for_model(&organs, AnatomyModel::Female).len(), 2);
    }

    #[test]
    fn discovered_filenames_resolve_to_body_systems() {
        // The whole point: a SPARQL-discovered filename feeds straight into the organ→system map.
        use wellfare_core::anatomy::body_system_for_organ;
        let organs = parse_ref_organs(SAMPLE_JSON);
        assert_eq!(
            body_system_for_organ(&organs[0].filename),
            Some("digestive")
        ); // liver
        assert_eq!(body_system_for_organ(&organs[1].filename), Some("urinary")); // kidney
        assert_eq!(
            body_system_for_organ(&organs[2].filename),
            Some("reproductive")
        ); // uterus
    }

    #[test]
    fn malformed_json_and_unsexed_assets_are_handled() {
        assert!(parse_ref_organs("not json").is_empty());
        // An asset with no -f-/-m- infix is skipped, not guessed.
        let no_sex = r#"{"results":{"bindings":[
          {"glb":{"type":"literal","value":"https://cdn.humanatlas.io/digital-objects/ref-organ/x/v1/assets/3d-vh-mystery.glb"}}
        ]}}"#;
        assert!(parse_ref_organs(no_sex).is_empty());
    }
}
