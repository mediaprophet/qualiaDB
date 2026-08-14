//! Produce a shippable **curated `.hmc` anatomy asset pack** for a model.
//!
//! The full CCF/HRA reference body is ~200–290 MB of GLB per model — too large to
//! bundle into a release. This builds a *curated* subset (a representative set of
//! organs across body systems, tens of MB) into a single `.hmc` bundle (see
//! [`qualia_core_db::bundle`]): each organ is a sealed `.10d` entry carrying an
//! [`AnatomyOrganMeta`] (system + approximate position + neutral colour). The
//! bundle is the artefact shipped in the desktop release resources and published
//! for the web demo, so a fresh install / the online demo renders a real body
//! with no per-user download.
//!
//! This is a **producer** (used by the `build_anatomy_pack` example / CI), not a
//! runtime path — it does blocking network I/O against the HRA endpoints.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use wellfare_core::anatomy::{normalize_organ_key, system_memberships_for_organ, AnatomyModel};

use qualia_core_db::bundle::BundleWriter;
use qualia_core_db::q42_volume::UnifiedVolumeBuilder;
use qualia_core_db::render::anatomy_pack::AnatomyOrganMeta;
use qualia_core_db::{NQuin, QUINS_PER_BLOCK};

use super::anatomy_body::{body_container, compile_body, organ_container, CompiledOrgan};
use super::ccf_resolver::{discover_ref_organs, fetch_glb, organs_for_model, HRA_SPARQL_ENDPOINT};

/// The default curated organ set — normalised base tokens (laterality/sex/`.glb`
/// stripped). A representative spread across body systems, kept small so the pack
/// is tens of MB. Discovery reports which of these were actually found.
pub const CURATED_ORGAN_TOKENS: &[&str] = &[
    // nervous
    "brain",
    "spinal-cord",
    // circulatory
    "heart",
    // respiratory
    "lung",
    "trachea",
    "larynx",
    "main-bronchus",
    // digestive
    "liver",
    "pancreas",
    "small-intestine",
    "large-intestine",
    "mouth",
    // urinary
    "kidney",
    "urinary-bladder",
    "ureter",
    // immune / lymphatic
    "spleen",
    "thymus",
    "lymph-node",
    // integumentary (skin — the outer body surface; the mixer defaults it muted so it
    // doesn't occlude the organs, and peels it on demand)
    "skin",
    // sensory
    "eye",
    // skeletal
    "pelvis",
    // reproductive (model-specific — each reference body matches only its own organs)
    "prostate",
    "uterus",
    "ovary",
    "vagina",
    "fallopian-tube",
];

/// A discovered organ: its CCF filename and its normalised base token.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredOrgan {
    pub filename: String,
    pub token: String,
}

/// Discover every reference organ for a model (filename + normalised token).
/// Useful for curating [`CURATED_ORGAN_TOKENS`] against what the HRA actually
/// serves.
pub fn discover_model_organs(model: AnatomyModel) -> Result<Vec<DiscoveredOrgan>, String> {
    let all =
        discover_ref_organs(HRA_SPARQL_ENDPOINT).map_err(|e| format!("SPARQL discovery: {e}"))?;
    Ok(organs_for_model(&all, model)
        .into_iter()
        .map(|o| DiscoveredOrgan {
            token: normalize_organ_key(&o.filename),
            filename: o.filename,
        })
        .collect())
}

/// The result of building a pack — honest about what packed and what curated
/// organ was not found/failed.
#[derive(Debug, Clone, Serialize)]
pub struct PackReport {
    pub model: String,
    pub out_path: String,
    pub organs_packed: usize,
    pub total_10d_bytes: usize,
    pub bundle_bytes: usize,
    /// Curated tokens that were requested but not discovered for this model.
    pub curated_not_found: Vec<String>,
    /// (filename, error) for organs that failed to fetch or compile.
    pub failed: Vec<(String, String)>,
    /// Organ keys packed, in order.
    pub packed_keys: Vec<String>,
    /// Size in bytes of the pack-level `.q42` provenance/semantics graph (carried in the
    /// bundle as `body.q42` and written beside the bundle as a linkable sidecar).
    pub q42_graph_bytes: usize,
    /// Number of quins (facts) in the pack `.q42` graph.
    pub q42_quins: usize,
    /// Path the `.q42` sidecar was written to.
    pub q42_sidecar_path: String,
}

/// Build a curated `.hmc` pack for `model` and write it to `out_path`.
///
/// `curated` is the set of normalised base tokens to include (defaults to
/// [`CURATED_ORGAN_TOKENS`] when `None`). Blocking network I/O.
pub fn build_anatomy_pack(
    model: AnatomyModel,
    out_path: impl AsRef<Path>,
    curated: Option<&[&str]>,
) -> Result<PackReport, String> {
    let out_path = out_path.as_ref();

    // Discover every reference organ for this model.
    let all =
        discover_ref_organs(HRA_SPARQL_ENDPOINT).map_err(|e| format!("SPARQL discovery: {e}"))?;
    let model_organs = organs_for_model(&all, model);

    // `Some(list)` selects a curated subset by normalised token; `None` builds the COMPLETE body —
    // every discovered reference organ for the model (skin, vasculature, and all). Since the renderer
    // now places organs by their true shared-space coordinates, no per-organ position curation is needed.
    let (selected, curated_not_found): (Vec<_>, Vec<String>) = match curated {
        Some(list) => {
            let sel: Vec<_> = model_organs
                .iter()
                .filter(|o| {
                    let key = normalize_organ_key(&o.filename);
                    list.iter().any(|t| key.as_str() == *t)
                })
                .cloned()
                .collect();
            let found: std::collections::BTreeSet<String> = sel
                .iter()
                .map(|o| normalize_organ_key(&o.filename))
                .collect();
            let missing: Vec<String> = list
                .iter()
                .filter(|t| !found.contains(**t))
                .map(|t| (*t).to_string())
                .collect();
            (sel, missing)
        }
        None => (model_organs.clone(), Vec::new()),
    };

    if selected.is_empty() {
        return Err(format!(
            "no reference organs discovered for {}",
            model.as_str()
        ));
    }

    // Fetch each selected GLB.
    let mut fetched: Vec<(String, Vec<u8>)> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for organ in &selected {
        match fetch_glb(&organ.glb_url) {
            Ok(bytes) => fetched.push((organ.filename.clone(), bytes)),
            Err(e) => failed.push((organ.filename.clone(), format!("fetch: {e}"))),
        }
    }

    // Compile the fetched GLBs to sealed `.10d`.
    let compiled = compile_body(model, &fetched);
    for (k, e) in &compiled.failed {
        failed.push((k.clone(), format!("compile: {e}")));
    }

    // Pack each compiled organ as a `.10d` entry with its render meta.
    let mut writer = BundleWriter::new();
    let mut total_10d_bytes = 0usize;
    let mut packed_keys: Vec<String> = Vec::new();
    for organ in &compiled.organs {
        // All systems this organ participates in (primary first) — so the pack supports colouring by the
        // primary system OR blending across memberships, and a condition on any member system lights it.
        let systems: Vec<String> = system_memberships_for_organ(&organ.organ_key)
            .into_iter()
            .map(|(s, _)| s.to_string())
            .collect();
        let meta = AnatomyOrganMeta {
            system: organ.system_id.clone(),
            label: normalize_organ_key(&organ.organ_key), // "3d-vh-m-heart.glb" → "heart"
            systems,
            position: position_for(&organ.organ_key),
            rgba: palette_for(&organ.system_id),
        };
        let bytes = organ.asset.container_10d.clone();
        total_10d_bytes += bytes.len();
        writer
            .add_file(organ.organ_key.clone(), "10d", bytes, Some(meta.to_cbor()))
            .map_err(|e| format!("bundle add {}: {e}", organ.organ_key))?;
        packed_keys.push(organ.organ_key.clone());
    }

    // Pack-level `.q42`: the body's provenance + organ→system semantic graph. It is the
    // growing semantic spine the copyright panel links to, and to which disease↔organ links
    // and — privately, client-side — the person's own conditions are later appended. Built
    // from the SAME hypermedia containers the desktop uses, so the pack's semantics are the
    // product's semantics (not a demo aside). Its source citations use the real CCF/HRA CDN
    // URLs each GLB was fetched from.
    let source_urls: HashMap<String, String> = selected
        .iter()
        .map(|o| (o.filename.clone(), o.glb_url.clone()))
        .collect();
    let (q42_bytes, q42_quins) = build_pack_q42(model, &compiled.organs, &source_urls);
    let q42_graph_bytes = q42_bytes.len();
    // Carried INSIDE the bundle (one attestable unit alongside the `.10d` meshes).
    writer
        .add_file("body.q42", "q42", q42_bytes.clone(), None)
        .map_err(|e| format!("bundle add body.q42: {e}"))?;

    let bundle = writer.build().map_err(|e| format!("bundle build: {e}"))?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create out dir: {e}"))?;
    }
    std::fs::write(out_path, &bundle).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    // Also write the same `.q42` as a standalone sidecar next to the bundle (e.g.
    // `anatomy-male.q42`) — a directly-linkable provenance/semantics file for the web demo,
    // byte-identical to the bundle's `body.q42` entry (one graph, two carriers, no drift).
    let q42_sidecar = out_path.with_extension("q42");
    std::fs::write(&q42_sidecar, &q42_bytes)
        .map_err(|e| format!("write {}: {e}", q42_sidecar.display()))?;

    Ok(PackReport {
        model: model.as_str().to_string(),
        out_path: out_path.display().to_string(),
        organs_packed: compiled.organs.len(),
        total_10d_bytes,
        bundle_bytes: bundle.len(),
        curated_not_found,
        failed,
        packed_keys,
        q42_graph_bytes,
        q42_quins,
        q42_sidecar_path: q42_sidecar.display().to_string(),
    })
}

/// Aggregate the compiled organs into a single pack-level `.q42` graph volume: each organ
/// becomes a hypermedia container (its sealed `.10d` ⊕ its CCF/HRA source with licence +
/// creator ⊕ topic/system/depiction descriptors), bundled into one **body** container. The
/// returned bytes are a valid unified v3 `.q42` (object-sorted blocks) carrying the full
/// provenance + organ→system semantics — the same graph the desktop builds. Returns the
/// bytes and the number of quins (facts) in the graph.
fn build_pack_q42(
    model: AnatomyModel,
    organs: &[CompiledOrgan],
    source_urls: &HashMap<String, String>,
) -> (Vec<u8>, usize) {
    let containers: Vec<_> = organs
        .iter()
        .map(|o| organ_container(o, model, source_urls.get(&o.organ_key).map(String::as_str)))
        .collect();
    let body = body_container(model, &containers);
    let quins = body.quins.len();
    (q42_bytes_from_graph(&body.quins, &body.lexicon), quins)
}

/// Serialise a quin graph + object-lexicon into unified v3 `.q42` bytes. Quins are sorted by
/// `object` and chunked into [`QUINS_PER_BLOCK`]-sized SuperBlocks so the volume's BIDX (which
/// the header advertises as object-sorted) is truthful and object-hash lookups resolve.
fn q42_bytes_from_graph(quins: &[NQuin], lexicon: &HashMap<u64, String>) -> Vec<u8> {
    let mut sorted = quins.to_vec();
    sorted.sort_by_key(|q| q.object);
    let mut builder = UnifiedVolumeBuilder::with_lex_map(lexicon)
        .expect("body Q42 lexicon entries fit the current Q42LEX format");
    for (seq, chunk) in sorted.chunks(QUINS_PER_BLOCK).enumerate() {
        builder
            .push_block(seq as u64, chunk)
            .expect("body Q42 graph is object-sorted");
    }
    builder.finish_to_bytes()
}

/// The shipped default linear RGBA for a body system (the person's σ-derived burden colour overrides it
/// at runtime). Delegates to the [`wellfare_core::anatomy`] **system registry** — the single source of
/// truth for the system palette — so a registered extension system carries its own colour into the pack
/// and no colour table drifts. Unknown/unregistered systems get the neutral swatch.
fn palette_for(system: &str) -> [f32; 4] {
    wellfare_core::anatomy::default_registry().color_of(system)
}

/// An approximate anatomical position `[x, y, z]` in 0..1 body space (x=right,
/// y=up, z=front) for assembling the whole body. Approximate placement — a
/// future pass can use real CCF spatial-placement transforms. Laterality
/// (`-l`/`-r`) nudges x so paired organs don't overlap.
fn position_for(filename: &str) -> [f32; 3] {
    let token = normalize_organ_key(filename);
    let [x, y, z] = match token.as_str() {
        "brain" => [0.50, 0.93, 0.50],
        "spinal-cord" => [0.50, 0.70, 0.44],
        "trachea" => [0.50, 0.74, 0.55],
        "thymus" => [0.50, 0.66, 0.55],
        "heart" => [0.50, 0.60, 0.55],
        "lung" => [0.42, 0.62, 0.50],
        "liver" => [0.57, 0.53, 0.52],
        "stomach" => [0.44, 0.52, 0.52],
        "spleen" => [0.60, 0.52, 0.44],
        "pancreas" => [0.50, 0.50, 0.46],
        "gallbladder" => [0.56, 0.51, 0.55],
        "kidney" => [0.50, 0.47, 0.40],
        "small-intestine" => [0.50, 0.42, 0.55],
        "large-intestine" => [0.50, 0.42, 0.60],
        "urinary-bladder" => [0.50, 0.33, 0.55],
        "larynx" => [0.50, 0.77, 0.55],
        "main-bronchus" => [0.50, 0.66, 0.50],
        "mouth" => [0.50, 0.85, 0.56],
        "ureter" => [0.50, 0.40, 0.42],
        "lymph-node" => [0.44, 0.68, 0.50],
        "eye" => [0.50, 0.90, 0.57],
        "pelvis" => [0.50, 0.35, 0.50],
        "skin" => [0.50, 0.50, 0.50],
        "prostate" => [0.50, 0.31, 0.50],
        "uterus" => [0.50, 0.34, 0.50],
        "ovary" => [0.50, 0.37, 0.45],
        "vagina" => [0.50, 0.29, 0.50],
        "fallopian-tube" => [0.50, 0.38, 0.45],
        _ => [0.50, 0.50, 0.50],
    };
    // Laterality nudge for paired organs (kidney-l/-r, lung-l/-r, …).
    let lower = filename.to_ascii_lowercase();
    let x = if lower.contains("-l.") || lower.contains("-left") {
        x - 0.09
    } else if lower.contains("-r.") || lower.contains("-right") {
        x + 0.09
    } else {
        x
    };
    [x, y, z]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pack-level `.q42` must carry the real provenance (licence + source citation) and the
    /// organ→system semantics — and open as a valid unified volume whose facts are queryable.
    /// This is the growing semantic spine the copyright panel links to.
    #[test]
    fn pack_q42_carries_provenance_and_system_semantics() {
        use qualia_core_db::q42_volume::{Q42Volume, Q42_MAGIC};

        // Compile two organs whose systems are covered by the map (real GLB parsing is proven in
        // render::assets; a stand-in OBJ triangle exercises the pack graph, not the GLB decoder).
        const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let organs = vec![
            ("3d-vh-m-lung.obj".to_string(), TRI_OBJ.to_vec()),
            (
                "3d-vh-m-blood-vasculature.obj".to_string(),
                TRI_OBJ.to_vec(),
            ),
        ];
        let body = compile_body(AnatomyModel::Male, &organs);
        assert_eq!(body.organs.len(), 2, "both organs mapped to a system");

        let mut urls = HashMap::new();
        urls.insert(
            "3d-vh-m-lung.obj".to_string(),
            "https://cdn.humanatlas.io/hra/lung.glb".to_string(),
        );
        let (q42, quin_count) = build_pack_q42(AnatomyModel::Male, &body.organs, &urls);
        assert!(quin_count > 0, "the graph has facts");
        assert!(q42.starts_with(&Q42_MAGIC), "produced a Q42 volume");

        // It opens as a valid unified volume and every fact round-trips.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &q42).unwrap();
        let vol = Q42Volume::open(tmp.path()).unwrap();
        let quins = vol.read_all_quins().unwrap();
        assert_eq!(
            quins.len(),
            quin_count,
            "every fact recoverable from the .q42"
        );

        // Resolve the string-valued objects through the embedded lexicon.
        let lex = vol.lex_view().unwrap();
        let vals: Vec<String> = quins
            .iter()
            .filter_map(|q| lex.lookup_hash(q.object).map(str::to_string))
            .collect();
        // Provenance: the licence and the real CDN source URL are in the graph.
        assert!(
            vals.iter().any(|v| v == "CC-BY-4.0"),
            "licence fact present: {vals:?}"
        );
        assert!(
            vals.iter().any(|v| v.contains("humanatlas.io")),
            "source citation present: {vals:?}"
        );
        // Organ→system: lung→respiratory and blood-vasculature→circulatory are both bound.
        assert!(
            vals.iter().any(|v| v == "respiratory"),
            "lung system present"
        );
        assert!(
            vals.iter().any(|v| v == "circulatory"),
            "vasculature system present"
        );
    }

    #[test]
    fn laterality_nudges_paired_organs_apart() {
        let l = position_for("3d-vh-f-kidney-l.glb");
        let r = position_for("3d-vh-f-kidney-r.glb");
        assert!(
            l[0] < r[0],
            "left kidney is left of right kidney: {l:?} {r:?}"
        );
        // Unpaired organ is centred.
        assert_eq!(position_for("3d-vh-m-heart.glb")[0], 0.50);
    }

    #[test]
    fn palette_covers_systems_with_neutral_fallback() {
        // Canonical ids from wellfare_core::anatomy::systems get real colours…
        assert_eq!(palette_for("circulatory")[0], 0.80);
        assert_eq!(palette_for("immune_lymphatic")[1], 0.82);
        // …and every organ we actually pack resolves to a non-default colour.
        for sys in [
            "nervous",
            "circulatory",
            "respiratory",
            "digestive",
            "urinary",
            "immune_lymphatic",
        ] {
            assert_ne!(
                palette_for(sys),
                [0.62, 0.66, 0.72, 1.0],
                "{sys} should have a colour"
            );
        }
        // Unknown system falls back to neutral.
        assert_eq!(palette_for("unknown-system"), [0.62, 0.66, 0.72, 1.0]);
    }
}
