//! Ingest the **BodyParts3D** FMA-keyed anatomy meshes — the open library that *completes* the body the
//! CCF/HRA reference organs cannot: 437 individual muscles, 251 bones, 99 nerves, endocrine + sense-organ
//! glands. Where CCF is viscera-only, BodyParts3D fills the muscular / skeletal / nervous / endocrine /
//! sensory systems.
//!
//! **Licence (precise):** the mesh files originate from the **BodyParts3D / Anatomography** project at
//! **lifesciencedb.jp** (© The Database Center for Life Science, DBCLS) and are licensed
//! **CC-BY-SA 2.1 Japan** (Attribution-**ShareAlike**). We retrieve them via a redistribution repo,
//! `github.com/Kevin-Mattheus-Moerman/BodyParts3D` — whose *own* `LICENSE` (MIT) covers only Moerman's
//! Julia code + OBJ→STL packaging, **not** the anatomy data: a permissive wrapper does not relicense the
//! upstream copyleft meshes. So the meshes remain CC-BY-SA and ship as a **separate, clearly-licensed
//! pack** (share-alike stays contained; the CCF CC-BY-4.0 pack stays permissive). Attribution + citation
//! are recorded per the database's terms ([`BP3D_ATTRIBUTION`], [`BP3D_CITATION`], [`BP3D_DATA_DOI`]).
//! Data version 3.0 / 20110915 (a single male reference model).
//!
//! **The join (Timothy's "organs are parts of systems" made real):** every mesh is keyed by an FMA id;
//! `conventional_part_of.txt` gives the part-of hierarchy with the *systems themselves* as FMA nodes
//! (`FMA72954` = muscular system, `FMA9668` = endocrine system, …). Walking a structure **up** the
//! part-of graph until it reaches a system root yields its system membership(s) — and a structure that
//! reaches several roots is genuinely multi-system (the diaphragm resolves to *both* muscular and
//! respiratory, straight from the ontology).
//!
//! This module is **pure** (parse + graph walk), unit-tested against fixtures; the live fetch + pack
//! producer are a `#[cfg(not(wasm32))]` transport layer at the bottom.

use std::collections::{HashMap, HashSet};

/// Raw base of the BodyParts3D fork we ingest (STL + the mapping files live under it).
pub const BP3D_RAW_BASE: &str =
    "https://raw.githubusercontent.com/Kevin-Mattheus-Moerman/BodyParts3D/main";
/// Repo-relative path of the STL directory (files are `FMA<id>.stl` / `BP<id>.stl`).
pub const BP3D_STL_DIR: &str = "assets/BodyParts3D_data/stl";
/// Repo-relative path of the id→English-name list (`"id"\ten` header, then tab-separated rows).
pub const BP3D_PARTS_LIST: &str = "assets/BodyParts3D_data/parts_list_e.txt";
/// Repo-relative path of the part-of hierarchy (`id, name, part id, part name` — `part` is part-of `id`).
pub const BP3D_PART_OF: &str = "assets/BodyParts3D_data/conventional_part_of.txt";
/// The BodyParts3D licence id, recorded in each mesh's provenance sidecar and the pack attribution.
pub const BP3D_LICENCE: &str = "CC-BY-SA-2.1-JP";
/// The **exact** attribution string the database's terms require (do not paraphrase away).
pub const BP3D_ATTRIBUTION: &str =
    "BodyParts3D, © The Database Center for Life Science licensed under CC Attribution-Share Alike 2.1 Japan";
/// The primary source of the mesh data (the originals; the GitHub repo is a redistribution mirror).
pub const BP3D_SOURCE_URL: &str = "https://lifesciencedb.jp/bp3d/";
/// The citation the database asks users of the content to include.
pub const BP3D_CITATION: &str = "Mitsuhashi N, Fujieda K, Tamura T, Kawamoto S, Takagi T, Okubo K. \
    BodyParts3D: 3D structure database for anatomical concepts. Nucleic Acids Res. 2009 Jan;37(Database issue):D782-5. \
    https://doi.org/10.1093/nar/gkn613";
/// The data-archive DOI for the BodyParts3D content.
pub const BP3D_DATA_DOI: &str = "https://doi.org/10.18908/lsdba.nbdc00837-000";

/// FMA ids of the anatomical **systems**, mapped to our body-system ids. A structure's part-of chain is
/// walked until it reaches one of these; the reached root(s) are the structure's system membership(s).
/// (Enumerated live from BodyParts3D's `parts_list_e.txt` — the 16 `*-system` nodes.)
static SYSTEM_ROOTS: &[(&str, &str)] = &[
    ("FMA7161", "circulatory"),        // cardiovascular system
    ("FMA7158", "respiratory"),        // respiratory system
    ("FMA7152", "digestive"),          // alimentary system
    ("FMA7157", "nervous"),            // nervous system
    ("FMA72954", "muscular"),          // muscular system
    ("FMA23881", "skeletal"),          // skeletal system
    ("FMA23878", "skeletal"),          // articular system (joints) → skeletal
    ("FMA61406", "skeletal"),          // skeletal system of free upper limb → skeletal
    ("FMA61409", "skeletal"),          // skeletal system of free lower limb → skeletal
    ("FMA9668", "endocrine"),          // endocrine system
    ("FMA74594", "immune_lymphatic"),  // lymphoid system
    ("FMA72979", "integumentary"),     // integumentary system
    ("FMA7159", "urinary"),            // urinary system
    ("FMA7160", "reproductive"),       // genital system
    ("FMA45664", "reproductive"),      // male genital system
    ("FMA78499", "sensory"),           // sense organ system
];

/// The body-system id for a system-root FMA id, if it is a known system root.
fn system_for_root(fma_id: &str) -> Option<&'static str> {
    SYSTEM_ROOTS.iter().find(|(f, _)| *f == fma_id).map(|(_, s)| *s)
}

/// Strip surrounding double-quotes and whitespace from a TSV cell (the header cells are quoted).
fn unquote(s: &str) -> &str {
    s.trim().trim_matches('"')
}

/// The BodyParts3D part-of hierarchy: id→name and part→wholes, for resolving a structure to its system(s).
pub struct Bp3dHierarchy {
    names: HashMap<String, String>,
    /// `part_id → [whole_id, …]` — the edges walked upward to reach a system root.
    part_to_wholes: HashMap<String, Vec<String>>,
}

impl Bp3dHierarchy {
    /// Build from the two mapping files' text: `parts_list_e.txt` (id→name) and
    /// `conventional_part_of.txt` (each row: whole `id` — its `part id`). Header rows are skipped.
    pub fn from_mapping(parts_list_txt: &str, part_of_txt: &str) -> Self {
        let mut names = HashMap::new();
        for line in parts_list_txt.lines() {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() >= 2 {
                let id = unquote(c[0]);
                if id != "id" && !id.is_empty() {
                    names.insert(id.to_string(), c[1].trim().to_string());
                }
            }
        }
        let mut part_to_wholes: HashMap<String, Vec<String>> = HashMap::new();
        for line in part_of_txt.lines() {
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() < 4 {
                continue;
            }
            let whole = unquote(c[0]);
            let part = unquote(c[2]);
            if whole == "id" || whole.is_empty() || part.is_empty() {
                continue; // header / malformed
            }
            part_to_wholes.entry(part.to_string()).or_default().push(whole.to_string());
        }
        Self { names, part_to_wholes }
    }

    /// The English anatomical name for a structure id.
    pub fn name(&self, id: &str) -> Option<&str> {
        self.names.get(id).map(String::as_str)
    }

    /// The **direct** part-of parents (wholes) of a structure — the immediate `partOf` edges for the
    /// ontology (as opposed to [`systems_for`](Self::systems_for), which walks all the way to a system).
    pub fn wholes_of(&self, id: &str) -> &[String] {
        self.part_to_wholes.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The body system(s) a structure belongs to — walk part→whole up to the system roots. A structure
    /// can reach several roots (the diaphragm is muscular **and** respiratory), so all are returned,
    /// sorted for determinism. Empty if it reaches no system (abstract / immaterial nodes).
    pub fn systems_for(&self, id: &str) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(cur) = stack.pop() {
            if !seen.insert(cur.clone()) {
                continue;
            }
            if let Some(sys) = system_for_root(&cur) {
                if !out.contains(&sys) {
                    out.push(sys);
                }
            }
            if let Some(wholes) = self.part_to_wholes.get(&cur) {
                for w in wholes {
                    stack.push(w.clone());
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// The raw STL URL for a BodyParts3D structure id (`FMA13295` → `…/stl/FMA13295.stl`).
pub fn stl_url(id: &str) -> String {
    format!("{BP3D_RAW_BASE}/{BP3D_STL_DIR}/{id}.stl")
}

/// Repo-relative path of the FMA is-a table (`FMAID,"Preferred Label",Parent FMAID`).
pub const BP3D_FMA_CSV: &str = "assets/BodyParts3D_data/FMA.csv";

/// Parse the FMA **is-a** CSV (`FMAID,"Preferred Label",Parent FMAID`) into a child→parent map, keyed in
/// the `FMA<id>` form (matching the STL filenames). The label may contain commas (it is quoted), so the
/// id and parent are taken as the first and last comma-separated fields — robust to embedded commas.
pub fn parse_fma_isa(csv: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in csv.lines() {
        let (first, last) = match (line.find(','), line.rfind(',')) {
            (Some(f), Some(l)) if l > f => (f, l),
            _ => continue,
        };
        let id = unquote(&line[..first]);
        let parent = unquote(&line[last + 1..]);
        if id.is_empty() || parent.is_empty() || id == "FMAID" {
            continue; // header / blank
        }
        // CSV ids are bare numbers; meshes are `FMA<num>` — normalise to the FMA-prefixed form.
        if id.bytes().all(|b| b.is_ascii_digit()) && parent.bytes().all(|b| b.is_ascii_digit()) {
            out.insert(format!("FMA{id}"), format!("FMA{parent}"));
        }
    }
    out
}

// ── Live fetch + pack producer (native only; blocking network I/O) ───────────────────────────────
#[cfg(not(target_arch = "wasm32"))]
mod producer {
    use super::*;
    use std::collections::{BTreeMap, HashSet};
    use std::path::Path;

    use qualia_core_db::bundle::BundleWriter;
    use qualia_core_db::container_10d::ProvenanceSidecar;
    use qualia_core_db::render::anatomy_pack::AnatomyOrganMeta;
    use qualia_core_db::render::compile_10d::compile_organ_asset;

    const HTTP_USER_AGENT: &str = "QualiaDB-anatomy/1.0";

    fn get_text(url: &str) -> Result<String, String> {
        let resp = reqwest::blocking::Client::new()
            .get(url)
            .header(reqwest::header::USER_AGENT, HTTP_USER_AGENT)
            .send()
            .map_err(|e| format!("GET {url}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("status {url}: {e}"))?;
        resp.text().map_err(|e| format!("body {url}: {e}"))
    }

    fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
        let resp = reqwest::blocking::Client::new()
            .get(url)
            .header(reqwest::header::USER_AGENT, HTTP_USER_AGENT)
            .send()
            .map_err(|e| format!("GET {url}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("status {url}: {e}"))?;
        Ok(resp.bytes().map_err(|e| format!("body {url}: {e}"))?.to_vec())
    }

    /// One available STL structure: its id and byte size (from the git-trees listing).
    #[derive(Debug, Clone)]
    pub struct Bp3dAsset {
        pub id: String,
        pub size: usize,
    }

    /// List every STL structure available in the repo (id + byte size) via the GitHub git-trees API.
    pub fn list_available_stl() -> Result<Vec<Bp3dAsset>, String> {
        let url = "https://api.github.com/repos/Kevin-Mattheus-Moerman/BodyParts3D/git/trees/main?recursive=1";
        let json = get_text(url)?;
        let v: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| format!("tree json: {e}"))?;
        let prefix = format!("{BP3D_STL_DIR}/");
        let mut out = Vec::new();
        if let Some(arr) = v.get("tree").and_then(|t| t.as_array()) {
            for e in arr {
                let path = e.get("path").and_then(|p| p.as_str()).unwrap_or("");
                if let Some(fname) = path.strip_prefix(&prefix) {
                    if let Some(id) = fname.strip_suffix(".stl") {
                        let size = e.get("size").and_then(|s| s.as_u64()).unwrap_or(0) as usize;
                        out.push(Bp3dAsset { id: id.to_string(), size });
                    }
                }
            }
        }
        Ok(out)
    }

    /// What to include from BodyParts3D — the **bandwidth control** (the full set is ~1.3 GB / 937 files).
    #[derive(Debug, Clone, Default)]
    pub struct Bp3dSelection {
        /// Only structures whose membership intersects these system ids (empty = every system).
        pub systems: Vec<String>,
        /// Cap the structure count (0 = no cap).
        pub max_structures: usize,
        /// Skip a structure whose STL exceeds this many bytes (0 = no cap) — e.g. drop the 79 MB
        /// whole-body composite blobs.
        pub max_stl_bytes: usize,
    }

    /// Honest report of a BodyParts3D pack build.
    #[derive(Debug, Clone)]
    pub struct Bp3dPackReport {
        pub out_path: String,
        pub structures_packed: usize,
        pub bundle_bytes: usize,
        pub total_stl_bytes: usize,
        /// (system_id, structure count) — the completeness this pack adds, per system.
        pub per_system: Vec<(String, usize)>,
        /// Size in bytes of the ontology `.q42` (concepts + is-a + part-of + system + geometry links).
        pub ontology_q42_bytes: usize,
        /// Number of quins (facts) in the ontology graph.
        pub ontology_quins: usize,
        /// Path the linkable `.q42` sidecar was written to (byte-identical to the bundle's `body.q42`).
        pub q42_sidecar_path: String,
        /// (structure id, error) for anything that failed to fetch or compile — never silently dropped.
        pub failed: Vec<(String, String)>,
    }

    /// Build a **separate, CC-BY-SA** `.qualia` pack of BodyParts3D structures that complete the body
    /// (the muscles/bones/glands/nerves CCF lacks). Each mesh is resolved to its system(s) via the
    /// part-of walk, compiled to a sealed `.10d` attested with the BodyParts3D licence, and packed with
    /// its full multi-system [`AnatomyOrganMeta`]. Blocking network I/O; honest about what failed.
    pub fn build_bodyparts3d_pack(
        selection: &Bp3dSelection,
        out_path: impl AsRef<Path>,
    ) -> Result<Bp3dPackReport, String> {
        let out_path = out_path.as_ref();

        // 1. The part-of hierarchy (id→name, part→whole) for resolving structures to systems.
        let parts = get_text(&format!("{BP3D_RAW_BASE}/{BP3D_PARTS_LIST}"))?;
        let part_of = get_text(&format!("{BP3D_RAW_BASE}/{BP3D_PART_OF}"))?;
        let hier = Bp3dHierarchy::from_mapping(&parts, &part_of);

        // 2. What's available, deterministically ordered, filtered by the selection.
        let mut avail = list_available_stl()?;
        avail.sort_by(|a, b| a.id.cmp(&b.id));
        let want: HashSet<&str> = selection.systems.iter().map(String::as_str).collect();
        let mut selected: Vec<(String, Vec<&'static str>)> = Vec::new();
        for a in &avail {
            if selection.max_stl_bytes > 0 && a.size > selection.max_stl_bytes {
                continue;
            }
            let systems = hier.systems_for(&a.id);
            if systems.is_empty() {
                continue; // abstract / immaterial — nothing to place
            }
            if !want.is_empty() && !systems.iter().any(|s| want.contains(s)) {
                continue;
            }
            selected.push((a.id.clone(), systems));
            if selection.max_structures > 0 && selected.len() >= selection.max_structures {
                break;
            }
        }
        if selected.is_empty() {
            return Err("no BodyParts3D structures matched the selection".to_string());
        }

        // 3. Fetch → compile (attested CC-BY-SA) → pack. Failures are reported, not dropped.
        let mut writer = BundleWriter::new();
        let mut failed: Vec<(String, String)> = Vec::new();
        let mut total_stl_bytes = 0usize;
        let mut per_system: BTreeMap<String, usize> = BTreeMap::new();
        let mut packed = 0usize;
        // Concepts (id + compiled `.10d` digest + systems) → the ontology `.q42` after the loop.
        let mut concepts: Vec<super::super::bodyparts3d_ontology::OntologyConcept> = Vec::new();
        for (id, systems) in &selected {
            let bytes = match get_bytes(&stl_url(id)) {
                Ok(b) => b,
                Err(e) => {
                    failed.push((id.clone(), e));
                    continue;
                }
            };
            total_stl_bytes += bytes.len();
            let primary = systems[0];
            let uri = format!("urn:bodyparts3d:{id}");
            // Attest each mesh with the BodyParts3D licence so the `.10d` passes the renderer's
            // fail-closed governance gate and travels with its CC-BY-SA provenance.
            let provenance = ProvenanceSidecar::new(uri.clone().into_bytes(), "model/stl", BP3D_LICENCE);
            match compile_organ_asset(&bytes, Some("stl"), &uri, "stl", Some(primary), None, Some(&provenance)) {
                Ok(asset) => {
                    let digest = asset.compiled_digest; // capture before container_10d is moved
                    let meta = AnatomyOrganMeta {
                        system: primary.to_string(),
                        label: hier.name(id).unwrap_or(id).to_string(), // human FMA name for the parts list
                        systems: systems.iter().map(|s| s.to_string()).collect(),
                        position: [0.5, 0.5, 0.5], // BodyParts3D meshes carry true coordinates; the renderer uses those
                        rgba: wellfare_core::anatomy::default_registry().color_of(primary),
                    };
                    if let Err(e) = writer.add_file(format!("{id}.10d"), "10d", asset.container_10d, Some(meta.to_cbor())) {
                        failed.push((id.clone(), format!("bundle add: {e}")));
                        continue;
                    }
                    for s in systems {
                        *per_system.entry(s.to_string()).or_default() += 1;
                    }
                    concepts.push(super::super::bodyparts3d_ontology::OntologyConcept {
                        id: id.clone(),
                        compiled_digest: digest,
                        systems: systems.iter().map(|s| s.to_string()).collect(),
                    });
                    packed += 1;
                }
                Err(e) => failed.push((id.clone(), format!("compile: {e}"))),
            }
        }
        if packed == 0 {
            return Err(format!("no BodyParts3D structures compiled (all {} failed)", failed.len()));
        }

        // The addressable ONTOLOGY: fetch the FMA is-a table and emit the `.q42` graph (OBO IRIs + house
        // aliases, is-a + part-of + system + geometry links) that cites the `.10d` meshes just packed.
        let fma_csv = get_text(&format!("{BP3D_RAW_BASE}/{BP3D_FMA_CSV}"))?;
        let isa = parse_fma_isa(&fma_csv);
        let (q42_bytes, ontology_quins) =
            super::super::bodyparts3d_ontology::ontology_q42_bytes(&concepts, &hier, &isa);
        let ontology_q42_bytes = q42_bytes.len();
        writer
            .add_file("body.q42", "q42", q42_bytes.clone(), None)
            .map_err(|e| format!("bundle add body.q42: {e}"))?;

        let bundle = writer.build().map_err(|e| format!("bundle build: {e}"))?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create out dir: {e}"))?;
        }
        std::fs::write(out_path, &bundle).map_err(|e| format!("write {}: {e}", out_path.display()))?;
        // A directly-linkable, byte-identical ontology `.q42` sidecar beside the bundle.
        let q42_sidecar = out_path.with_extension("q42");
        std::fs::write(&q42_sidecar, &q42_bytes)
            .map_err(|e| format!("write {}: {e}", q42_sidecar.display()))?;

        Ok(Bp3dPackReport {
            out_path: out_path.display().to_string(),
            structures_packed: packed,
            bundle_bytes: bundle.len(),
            total_stl_bytes,
            per_system: per_system.into_iter().collect(),
            ontology_q42_bytes,
            ontology_quins,
            q42_sidecar_path: q42_sidecar.display().to_string(),
            failed,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use producer::{build_bodyparts3d_pack, list_available_stl, Bp3dAsset, Bp3dPackReport, Bp3dSelection};

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal fixture mirroring the real files' format (tab-separated, quoted header).
    const PARTS: &str = "\"id\"\ten\n\
        FMA20394\thuman body\n\
        FMA72954\tmuscular system\n\
        FMA7158\trespiratory system\n\
        FMA9668\tendocrine system\n\
        FMA13295\tdiaphragm\n\
        FMA_BICEPS\tbiceps brachii\n\
        FMA_THYROID\tthyroid gland\n";
    // Rows: whole `id` — its `part id`. The diaphragm is a part of BOTH muscular and respiratory systems.
    const PART_OF: &str = "\"id\"\tname\tpart id\tpart name\n\
        FMA20394\thuman body\tFMA72954\tmuscular system\n\
        FMA20394\thuman body\tFMA7158\trespiratory system\n\
        FMA20394\thuman body\tFMA9668\tendocrine system\n\
        FMA72954\tmuscular system\tFMA13295\tdiaphragm\n\
        FMA7158\trespiratory system\tFMA13295\tdiaphragm\n\
        FMA72954\tmuscular system\tFMA_BICEPS\tbiceps brachii\n\
        FMA9668\tendocrine system\tFMA_THYROID\tthyroid gland\n";

    #[test]
    fn maps_names_and_walks_structures_to_their_systems() {
        let h = Bp3dHierarchy::from_mapping(PARTS, PART_OF);
        assert_eq!(h.name("FMA13295"), Some("diaphragm"));
        assert_eq!(h.name("FMA_THYROID"), Some("thyroid gland"));
        // A single-system structure resolves to one system…
        assert_eq!(h.systems_for("FMA_BICEPS"), vec!["muscular"]);
        assert_eq!(h.systems_for("FMA_THYROID"), vec!["endocrine"]);
        // …and a genuine dual-role structure resolves to BOTH (straight from the ontology).
        assert_eq!(h.systems_for("FMA13295"), vec!["muscular", "respiratory"]);
    }

    #[test]
    fn system_roots_themselves_resolve_and_unknowns_are_empty() {
        let h = Bp3dHierarchy::from_mapping(PARTS, PART_OF);
        // A system root resolves to itself.
        assert_eq!(h.systems_for("FMA72954"), vec!["muscular"]);
        // An id with no path to any system → no membership (reported empty, never guessed).
        assert!(h.systems_for("FMA_NOT_A_THING").is_empty());
    }

    #[test]
    fn every_system_root_maps_to_a_real_body_system() {
        // Guard: each FMA system root maps to an id the registry actually knows.
        let reg = wellfare_core::anatomy::default_registry();
        for (fma, sys) in SYSTEM_ROOTS {
            assert!(reg.get(sys).is_some(), "root {fma} → unknown system id {sys}");
        }
    }

    #[test]
    fn parse_fma_isa_reads_child_to_parent_even_with_commas_in_labels() {
        let csv = "\"FMAID\",\"Preferred Label\",\"Parent FMAID\"\n\
            13295,\"Diaphragm\",9909\n\
            7163,\"Skin, layer of body\",72979\n";
        let isa = parse_fma_isa(csv);
        assert_eq!(isa.get("FMA13295"), Some(&"FMA9909".to_string()));
        // A label containing a comma is still parsed (id = first field, parent = last field).
        assert_eq!(isa.get("FMA7163"), Some(&"FMA72979".to_string()));
        assert_eq!(isa.len(), 2, "header row skipped");
    }

    #[test]
    fn wholes_of_returns_direct_part_of_parents() {
        let h = Bp3dHierarchy::from_mapping(PARTS, PART_OF);
        // The diaphragm is a direct part of both the muscular and respiratory systems (fixture row order).
        assert_eq!(h.wholes_of("FMA13295"), &["FMA72954".to_string(), "FMA7158".to_string()]);
        assert!(h.wholes_of("FMA_UNKNOWN").is_empty());
    }

    #[test]
    fn stl_url_is_the_raw_github_path() {
        assert_eq!(
            stl_url("FMA13295"),
            "https://raw.githubusercontent.com/Kevin-Mattheus-Moerman/BodyParts3D/main/assets/BodyParts3D_data/stl/FMA13295.stl"
        );
    }
}
