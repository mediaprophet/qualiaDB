use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, HashMap};

/// Represents a simple event from the streaming YAML lexer.
/// In a zero-alloc edge system, these would stream directly from a `&[u8]` buffer.
#[derive(Debug)]
pub enum YamlToken<'a> {
    MapStart,
    MapEnd,
    ListStart,
    ListEnd,
    Key(&'a str),
    ValueString(&'a str),
    ValueInt(i64),
}

/// A lightweight, state-machine based lexer for yaml-ld-q42.
/// Designed to parse without full document materialisation, avoiding `Vec` or `String` allocs
/// during the hot-path traversal of pane manifests.
pub struct YamlStreamingLexer<'a> {
    buffer: &'a [u8],
    cursor: usize,
}

impl<'a> YamlStreamingLexer<'a> {
    /// Creates a default `YamlStreamingLexer`.
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }

    fn skip_ws_and_comments(&mut self) {
        while self.cursor < self.buffer.len() {
            let b = self.buffer[self.cursor];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.cursor += 1;
                continue;
            }
            if b == b'#' {
                while self.cursor < self.buffer.len() && self.buffer[self.cursor] != b'\n' {
                    self.cursor += 1;
                }
                continue;
            }
            break;
        }
    }

    /// Advance the lexer to the next semantic token.
    pub fn next_token(&mut self) -> Option<YamlToken<'a>> {
        self.skip_ws_and_comments();
        if self.cursor >= self.buffer.len() {
            return None;
        }
        if self.buffer[self.cursor] == b'-' {
            self.cursor += 1;
            return Some(YamlToken::ListStart);
        }
        let start = self.cursor;
        while self.cursor < self.buffer.len() {
            let b = self.buffer[self.cursor];
            if b == b':' || b.is_ascii_whitespace() {
                break;
            }
            self.cursor += 1;
        }
        if self.cursor >= self.buffer.len() || self.buffer[self.cursor] != b':' {
            self.cursor = self.buffer.len();
            return None;
        }
        let key = std::str::from_utf8(&self.buffer[start..self.cursor]).ok()?;
        self.cursor += 1;
        Some(YamlToken::Key(key))
    }
}

/// A fully structured Webizen Studio workspace, typically deserialized from yaml-ld-q42.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebizenWorkspace {
    #[serde(rename = "@id", default)]
    pub id: Option<String>,
    #[serde(default)]
    pub theme_tokens: BTreeMap<String, String>,
    pub pages: Vec<Page>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    #[serde(default)]
    pub url_path: String,
    pub name: String,
    #[serde(default)]
    pub panes: Vec<Pane>,
    #[serde(default)]
    pub views: Vec<ManifoldView>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    pub component_id: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    #[serde(default)]
    pub data_bindings: Vec<String>,
}

/// Phase 5 Manifold World view definition (`QappView` wire representation).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifoldView {
    pub manifold: String,
    #[serde(default = "default_view_kind")]
    pub kind: String,
    #[serde(default = "default_view_sensitivity")]
    pub sensitivity: String,
    #[serde(default)]
    pub requires_attestation: bool,
}

fn default_view_kind() -> String {
    "Scene3D".into()
}

fn default_view_sensitivity() -> String {
    "Public".into()
}

/// Synth project & asset metadata extracted from a `yaml-ld-q42/asset` manifest (`manifest.q42.yaml`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct YamlAssetProjectConfig {
    pub id: String,
    pub name: String,
    pub brief: String,
    pub domain: String,
    pub fiction: bool,
    pub quality_profile: String,
    pub working: String,
    pub delivered: String,
    pub sources: Vec<String>,
}

/// Result of compiling a yaml-ld-q42 document into NQuins + Q42LEX lexicon map.
pub type HcfCompileResult = (Vec<NQuin>, HashMap<u64, String>);

fn lex_insert(lexicon: &mut HashMap<u64, String>, surface: &str) -> u64 {
    let h = q_hash(surface);
    lexicon.entry(h).or_insert_with(|| surface.to_string());
    h
}

fn emit_spo(
    quins: &mut Vec<NQuin>,
    lexicon: &mut HashMap<u64, String>,
    subject: u64,
    pred_iri: &str,
    object_surface: &str,
    namespace: u64,
    lamport: u64,
) {
    let predicate = lex_insert(lexicon, pred_iri);
    let object = lex_insert(lexicon, object_surface);
    quins.push(NQuin {
        subject,
        predicate,
        object,
        context: namespace,
        metadata: lamport << 32,
        parity: subject ^ predicate ^ object ^ namespace,
    });
}

fn yaml_string<'a>(v: &'a YamlValue) -> Option<&'a str> {
    v.as_str()
}

fn yaml_string_owned(v: &YamlValue) -> Option<String> {
    match v {
        YamlValue::String(s) => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Parses a `yaml-ld-q42/workspace` byte stream (including 2D `panes` and Phase 5 Manifold `views`),
/// populates the `Q42LEX` surface map, and compiles into 48-byte `NQuin` records.
pub fn compile_workspace_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport_clock: u64,
) -> Result<HcfCompileResult, &'static str> {
    let mut lexer = YamlStreamingLexer::new(yaml_bytes);
    while lexer.next_token().is_some() {}

    let workspace: WebizenWorkspace =
        serde_yaml::from_slice(yaml_bytes).map_err(|_| "Failed to parse yaml-ld-q42 payload")?;

    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();

    let ws_id = workspace
        .id
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("q42:WebizenWorkspaceRoot");
    let ws_subject = lex_insert(&mut lexicon, ws_id);
    emit_spo(
        &mut quins,
        &mut lexicon,
        ws_subject,
        "rdf:type",
        "WebizenWorkspace",
        namespace,
        lamport_clock,
    );

    for (token_key, token_val) in &workspace.theme_tokens {
        let token_pred = format!("q42:themeToken:{token_key}");
        emit_spo(
            &mut quins,
            &mut lexicon,
            ws_subject,
            &token_pred,
            token_val,
            namespace,
            lamport_clock,
        );
    }

    let pred_pane = lex_insert(&mut lexicon, "q42:SystemPaneState");
    let pred_page = lex_insert(&mut lexicon, "q42:SystemPageDef");

    for (pi, page) in workspace.pages.into_iter().enumerate() {
        let page_id_surface = if page.url_path.trim().is_empty() {
            format!("{ws_id}#page-{pi}")
        } else {
            page.url_path.clone()
        };
        let page_hash = lex_insert(&mut lexicon, &page_id_surface);
        let page_name_hash = lex_insert(&mut lexicon, &page.name);

        quins.push(NQuin {
            subject: page_hash,
            predicate: pred_page,
            object: page_name_hash,
            context: namespace,
            metadata: lamport_clock << 32,
            parity: page_hash ^ pred_page ^ page_name_hash ^ namespace,
        });

        for pane in page.panes {
            let packed_layout: u64 = ((pane.x as u64) << 24)
                | ((pane.y as u64) << 16)
                | ((pane.w as u64) << 8)
                | (pane.h as u64);

            let metadata = packed_layout | (lamport_clock << 32);
            let subject = lex_insert(&mut lexicon, &pane.component_id);

            quins.push(NQuin {
                subject,
                predicate: pred_pane,
                object: page_hash,
                context: namespace,
                metadata,
                parity: subject ^ pred_pane ^ page_hash ^ namespace,
            });

            for binding in &pane.data_bindings {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    subject,
                    "q42:paneDataBinding",
                    binding,
                    namespace,
                    lamport_clock,
                );
            }
        }

        for (vi, view) in page.views.into_iter().enumerate() {
            let view_id = format!("{page_id_surface}#view-{vi}");
            let view_subject = lex_insert(&mut lexicon, &view_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                page_hash,
                "q42:ManifoldViewDef",
                &view_id,
                namespace,
                lamport_clock,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                view_subject,
                "q42:viewManifold",
                &view.manifold,
                namespace,
                lamport_clock,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                view_subject,
                "q42:viewKind",
                &view.kind,
                namespace,
                lamport_clock,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                view_subject,
                "q42:viewSensitivity",
                &view.sensitivity,
                namespace,
                lamport_clock,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                view_subject,
                "q42:viewRequiresAttestation",
                if view.requires_attestation {
                    "true"
                } else {
                    "false"
                },
                namespace,
                lamport_clock,
            );
        }
    }

    Ok((quins, lexicon))
}

/// Backward-compatible workspace compiler returning `Vec<NQuin>`.
pub fn compile_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport_clock: u64,
) -> Result<Vec<NQuin>, &'static str> {
    compile_workspace_yaml_ld_to_quins(yaml_bytes, namespace, lamport_clock).map(|(quins, _)| quins)
}

/// Detect `@type: HypermediaDocument` (string or list containing it).
fn yaml_type_is_hypermedia_document(value: &YamlValue) -> bool {
    yaml_has_type_matching(value, type_str_is_hypermedia)
}

fn type_str_is_hypermedia(s: &str) -> bool {
    s == "HypermediaDocument"
        || s.ends_with(":HypermediaDocument")
        || s.ends_with("/HypermediaDocument")
        || s.ends_with("#HypermediaDocument")
}

/// Detect `@type: HypermediaAssetManifest` or asset/dataset types.
fn yaml_type_is_asset_manifest(value: &YamlValue) -> bool {
    yaml_has_type_matching(value, type_str_is_asset_manifest)
}

fn type_str_is_asset_manifest(s: &str) -> bool {
    matches!(
        s,
        "HypermediaAssetManifest"
            | "PoiesisArtefact"
            | "SpeciesEmbodiment"
            | "dcat:Dataset"
            | "made:Artefact"
            | "bio:DarwinianTaxon"
    ) || s.ends_with(":HypermediaAssetManifest")
        || s.ends_with("#HypermediaAssetManifest")
        || s.ends_with(":Artefact")
        || s.ends_with(":DarwinianTaxon")
}

fn yaml_has_type_matching(value: &YamlValue, pred: fn(&str) -> bool) -> bool {
    match value.get("@type") {
        Some(YamlValue::String(s)) => pred(s),
        Some(YamlValue::Sequence(seq)) => {
            seq.iter().any(|v| v.as_str().map(pred).unwrap_or(false))
        }
        _ => false,
    }
}

/// True when `content` is a non-empty sequence (HCF sections/blocks).
fn yaml_has_content_sections(value: &YamlValue) -> bool {
    value
        .get("content")
        .and_then(YamlValue::as_sequence)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// True when top-level `pages` is a non-empty sequence (WebizenWorkspace).
fn yaml_has_workspace_pages(value: &YamlValue) -> bool {
    value
        .get("pages")
        .and_then(YamlValue::as_sequence)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// True when asset-manifest keys (`distributions`, `lods`, `components`, `organs`, `project`, `computational_strata`, or `archetype_ref`) are present.
fn yaml_has_asset_sections(value: &YamlValue) -> bool {
    value.get("distributions").is_some()
        || value.get("lods").is_some()
        || value.get("components").is_some()
        || value.get("organs").is_some()
        || value.get("project").is_some()
        || value.get("instance_state_64").is_some()
        || value.get("computational_strata").is_some()
        || value.get("archetype_ref").is_some()
        || value.get("coordinates_10d").is_some()
        || value
            .get("schema")
            .and_then(YamlValue::as_str)
            .map(|s| s.starts_with("synth.project/") || s.starts_with("yaml-ld-q42/asset"))
            .unwrap_or(false)
}

/// Compile an HCF HypermediaDocument (yaml-ld-q42 authoring layer) into Quins + lexicon.
pub fn compile_hcf_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport: u64,
) -> Result<HcfCompileResult, String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse HCF yaml-ld-q42: {e}"))?;

    if !yaml_type_is_hypermedia_document(&root) && !yaml_has_content_sections(&root) {
        return Err(
            "not an HCF HypermediaDocument: need @type HypermediaDocument or content: sections"
                .into(),
        );
    }

    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();

    let doc_id_surface = root
        .get("@id")
        .and_then(yaml_string)
        .unwrap_or("hcf:anonymous");
    let doc_subject = lex_insert(&mut lexicon, doc_id_surface);

    emit_spo(
        &mut quins,
        &mut lexicon,
        doc_subject,
        "rdf:type",
        "HypermediaDocument",
        namespace,
        lamport,
    );

    for (key, pred) in [
        ("title", "hcf:title"),
        ("date", "hcf:date"),
        ("author", "hcf:author"),
        ("summary", "hcf:summary"),
        ("markdown", "hcf:markdown"),
        ("licence", "hcf:licence"),
        ("license", "hcf:licence"),
    ] {
        if let Some(v) = root.get(key).and_then(yaml_string_owned) {
            emit_spo(
                &mut quins,
                &mut lexicon,
                doc_subject,
                pred,
                &v,
                namespace,
                lamport,
            );
        }
    }

    for (list_key, pred) in [("tags", "hcf:tag"), ("depicts", "hcf:depictsAsset")] {
        if let Some(items) = root.get(list_key) {
            match items {
                YamlValue::String(s) => {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        doc_subject,
                        pred,
                        s,
                        namespace,
                        lamport,
                    );
                }
                YamlValue::Sequence(seq) => {
                    for item in seq {
                        if let Some(s) = yaml_string_owned(item) {
                            emit_spo(
                                &mut quins,
                                &mut lexicon,
                                doc_subject,
                                pred,
                                &s,
                                namespace,
                                lamport,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(YamlValue::Sequence(content)) = root.get("content") {
        for (si, section) in content.iter().enumerate() {
            let section_id = section
                .get("@id")
                .and_then(yaml_string)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{doc_id_surface}#section-{si}"));
            let section_subject = lex_insert(&mut lexicon, &section_id);

            emit_spo(
                &mut quins,
                &mut lexicon,
                doc_subject,
                "hcf:hasSection",
                &section_id,
                namespace,
                lamport,
            );

            if let Some(sec_type) = section.get("@type").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "rdf:type",
                    &sec_type,
                    namespace,
                    lamport,
                );
            }

            if let Some(heading) = section.get("heading").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:heading",
                    &heading,
                    namespace,
                    lamport,
                );
            }
            if let Some(md) = section.get("markdown").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:markdown",
                    &md,
                    namespace,
                    lamport,
                );
            }
            if let Some(text) = section.get("text").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:text",
                    &text,
                    namespace,
                    lamport,
                );
            }

            if let Some(YamlValue::Sequence(blocks)) = section.get("blocks") {
                for (bi, block) in blocks.iter().enumerate() {
                    compile_hcf_block(
                        &mut quins,
                        &mut lexicon,
                        section_subject,
                        &section_id,
                        bi,
                        block,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    Ok((quins, lexicon))
}

fn compile_hcf_block(
    quins: &mut Vec<NQuin>,
    lexicon: &mut HashMap<u64, String>,
    section_subject: u64,
    section_id: &str,
    bi: usize,
    block: &YamlValue,
    namespace: u64,
    lamport: u64,
) {
    let block_id = block
        .get("@id")
        .and_then(yaml_string)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{section_id}#block-{bi}"));
    let block_subject = lex_insert(lexicon, &block_id);

    emit_spo(
        quins,
        lexicon,
        section_subject,
        "hcf:hasBlock",
        &block_id,
        namespace,
        lamport,
    );

    if let Some(block_type) = block.get("@type").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "rdf:type",
            &block_type,
            namespace,
            lamport,
        );
    }

    if let Some(text) = block.get("text").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "hcf:text",
            &text,
            namespace,
            lamport,
        );
    }
    if let Some(md) = block.get("markdown").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "hcf:markdown",
            &md,
            namespace,
            lamport,
        );
    }
    if let Some(asset_ref) = block.get("asset").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "hcf:depictsAsset",
            &asset_ref,
            namespace,
            lamport,
        );
    }
}

/// Compile a `yaml-ld-q42/asset` (`@type: HypermediaAssetManifest`) document into NQuins + Q42LEX lexicon.
///
/// Unifies `synth.project.json`, DCAT `ontology/index.ttl`, and `dist/lods/lod-manifest.json`
/// into a single human-readable YAML-LD manifest that compiles directly into `dist/index.q42`.
pub fn compile_asset_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport: u64,
) -> Result<HcfCompileResult, String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse asset yaml-ld-q42: {e}"))?;

    if !yaml_type_is_asset_manifest(&root) && !yaml_has_asset_sections(&root) {
        return Err(
            "not a HypermediaAssetManifest: need @type HypermediaAssetManifest or asset sections (project/distributions/lods/components)"
                .into(),
        );
    }

    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();

    let fallback_slug = root
        .get("name")
        .or_else(|| root.get("slug"))
        .and_then(yaml_string)
        .unwrap_or("anonymous-asset");
    let fallback_id = format!("urn:qualia:asset:{fallback_slug}");
    let asset_id_surface = root
        .get("@id")
        .and_then(yaml_string)
        .unwrap_or(&fallback_id);
    let asset_subject = lex_insert(&mut lexicon, asset_id_surface);

    // Emit declared @type values plus canonical HypermediaAssetManifest & dcat:Dataset
    emit_spo(
        &mut quins,
        &mut lexicon,
        asset_subject,
        "rdf:type",
        "HypermediaAssetManifest",
        namespace,
        lamport,
    );
    emit_spo(
        &mut quins,
        &mut lexicon,
        asset_subject,
        "rdf:type",
        "dcat:Dataset",
        namespace,
        lamport,
    );
    match root.get("@type") {
        Some(YamlValue::String(t)) => {
            if t != "HypermediaAssetManifest" && t != "dcat:Dataset" {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    "rdf:type",
                    t,
                    namespace,
                    lamport,
                );
            }
        }
        Some(YamlValue::Sequence(seq)) => {
            for item in seq {
                if let Some(t) = item.as_str() {
                    if t != "HypermediaAssetManifest" && t != "dcat:Dataset" {
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            asset_subject,
                            "rdf:type",
                            t,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
        }
        _ => {}
    }

    // DID / sameAs / identifier
    if let Some(did) = root.get("did").and_then(yaml_string_owned) {
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "owl:sameAs",
            &did,
            namespace,
            lamport,
        );
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "dct:identifier",
            &did,
            namespace,
            lamport,
        );
    }
    emit_spo(
        &mut quins,
        &mut lexicon,
        asset_subject,
        "dct:identifier",
        asset_id_surface,
        namespace,
        lamport,
    );

    // Top-level scalar metadata
    for (key, pred) in [
        ("name", "q42:slug"),
        ("slug", "q42:slug"),
        ("title", "dct:title"),
        ("brief", "dct:description"),
        ("domain", "q42:domain"),
        ("fiction", "q42:fiction"),
        ("quality_profile", "q42:qualityProfile"),
        ("landing_page", "dcat:landingPage"),
        ("licence_policy", "odrl:hasPolicy"),
        ("license_policy", "odrl:hasPolicy"),
        ("epistemic_zone", "q42:epistemicZone"),
        ("embodiment_mode", "q42:embodimentMode"),
    ] {
        if let Some(val) = root.get(key).and_then(yaml_string_owned) {
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                pred,
                &val,
                namespace,
                lamport,
            );
        }
    }

    // Multilingual UTF-8 labels (`labels: { en: "...", rer: "...", zh: "..." }`)
    if let Some(YamlValue::Mapping(labels)) = root.get("labels") {
        for (lang_k, label_v) in labels {
            if let (Some(lang_str), Some(label_str)) = (lang_k.as_str(), yaml_string_owned(label_v))
            {
                let tagged = format!("{label_str}@{lang_str}");
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    "rdfs:label",
                    &tagged,
                    namespace,
                    lamport,
                );
            }
        }
    }

    // Primary 10D Computational Coordinates (`coordinates_10d: [q, v, w, x, y, z, t, α, μ, σ]`)
    const AXIS_NAMES_10D: [&str; 10] = ["q", "v", "w", "x", "y", "z", "t", "alpha", "mu", "sigma"];
    if let Some(coords) = root.get("coordinates_10d") {
        match coords {
            YamlValue::Sequence(seq) => {
                for (idx, val) in seq.iter().take(10).enumerate() {
                    if let Some(v_str) = yaml_string_owned(val) {
                        let pred = format!("q42:coord10d:{}", AXIS_NAMES_10D[idx]);
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            asset_subject,
                            &pred,
                            &v_str,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
            YamlValue::Mapping(map) => {
                for (k, v) in map {
                    if let (Some(k_str), Some(v_str)) = (k.as_str(), yaml_string_owned(v)) {
                        let pred = format!("q42:coord10d:{k_str}");
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            asset_subject,
                            &pred,
                            &v_str,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Mode-B External Archetype Reference (`archetype_ref` -> `FIELD_KIND_ARCHETYPE_REF = 0x0301`)
    if let Some(aref) = root.get("archetype_ref") {
        let aref_id = aref
            .get("@id")
            .or_else(|| aref.get("id"))
            .and_then(yaml_string)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{asset_id_surface}#archetype-ref"));
        let aref_subj = lex_insert(&mut lexicon, &aref_id);
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "q42:hasArchetypeRef",
            &aref_id,
            namespace,
            lamport,
        );
        emit_spo(
            &mut quins,
            &mut lexicon,
            aref_subj,
            "rdf:type",
            "q42:ArchetypeRef",
            namespace,
            lamport,
        );
        for (k, pred) in [
            ("digest", "q42:archetypeDigest"),
            ("digest_alg", "q42:digestAlgorithm"),
            ("entity_id", "q42:archetypeEntityId"),
            ("taxon_or_class_iri", "q42:taxonOrClassIri"),
            ("taxon_iri", "q42:taxonOrClassIri"),
            ("subresource_key", "q42:subresourceKey"),
            ("seed", "q42:proceduralSeed"),
            ("age_days", "q42:ageDays"),
            ("stage", "q42:ontogeneticStage"),
            ("uniform_scale", "q42:uniformScale"),
        ] {
            if let Some(v) = aref.get(k).and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    aref_subj,
                    pred,
                    &v,
                    namespace,
                    lamport,
                );
            }
        }
    }

    // 7-Layer Multi-Scale Nested Computational Strata (`computational_strata` -> `FIELD_KIND_MULTISCALE_STRATA = 0x0304`)
    if let Some(YamlValue::Sequence(strata)) = root.get("computational_strata") {
        for (si, st) in strata.iter().enumerate() {
            let st_id = st
                .get("@id")
                .or_else(|| st.get("id"))
                .and_then(yaml_string)
                .map(|s| {
                    if s.contains(':') {
                        s.to_string()
                    } else {
                        format!("{asset_id_surface}#stratum-{s}")
                    }
                })
                .unwrap_or_else(|| format!("{asset_id_surface}#stratum-{si}"));
            let st_subj = lex_insert(&mut lexicon, &st_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                "q42:hasComputationalStratum",
                &st_id,
                namespace,
                lamport,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                st_subj,
                "rdf:type",
                "q42:ComputationalStratumNode",
                namespace,
                lamport,
            );
            for (k, pred) in [
                ("id", "dct:identifier"),
                ("level", "q42:stratumLevel"),
                ("stratum_level", "q42:stratumLevel"),
                ("kind", "q42:constituentKind"),
                ("constituent_kind", "q42:constituentKind"),
                ("regime", "q42:interactionRegime"),
                ("interaction_regime", "q42:interactionRegime"),
                ("embodiment_state", "q42:embodimentState"),
                ("parent", "q42:parentStratum"),
                ("taxon_iri", "q42:taxonOrOntologyIri"),
                ("ontology_iri", "q42:taxonOrOntologyIri"),
                ("archetype_ref", "q42:stratumArchetypeRef"),
                ("scale_m", "q42:characteristicScaleMeters"),
                ("density_per_m3", "q42:densityPerCubicMeter"),
                ("metabolic_flux_w", "q42:metabolicFluxWatts"),
                ("quantum_regime", "q42quantum:quantumRegime"),
                ("hamiltonian_operator", "q42quantum:hamiltonianOperator"),
                ("coherence_time_ps", "q42quantum:dephasingTimePicoseconds"),
                ("t2_dephasing_ps", "q42quantum:dephasingTimePicoseconds"),
                ("t1_relaxation_ps", "q42quantum:relaxationTimePicoseconds"),
                ("entanglement_entropy", "q42quantum:vonNeumannEntropy"),
                ("tunneling_splitting_mev", "q42quantum:tunnelingSplittingMev"),
                ("berry_chern_number", "q42quantum:chernInvariant"),
                ("hartree_energy", "q42chem:hartreeEnergy"),
                ("homo_lumo_gap_ev", "q42chem:homoLumoGapEv"),
                ("dipole_debye", "q42chem:dipoleMomentDebye"),
                ("quantum_basis", "q42chem:gtoBasisSet"),
                ("xc_functional", "q42chem:xcFunctional"),
                ("qpu_nquin", "q42chem:qForgeStateVectorNQuin"),
                ("n3_rule", "q42logic:governedByN3Rule"),
                ("shacl_shape", "q42logic:validatedByShaclShape"),
                ("belnap_status", "q42logic:belnapTruthStatus"),
                ("deontic_mode", "q42logic:deonticModality"),
            ] {
                if let Some(v) = st.get(k).and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        st_subj,
                        pred,
                        &v,
                        namespace,
                        lamport,
                    );
                }
            }
            if let Some(YamlValue::Mapping(st_labels)) = st.get("labels") {
                for (lang_k, label_v) in st_labels {
                    if let (Some(lang_str), Some(label_str)) =
                        (lang_k.as_str(), yaml_string_owned(label_v))
                    {
                        let tagged = format!("{label_str}@{lang_str}");
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            st_subj,
                            "rdfs:label",
                            &tagged,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
        }
    }

    // Synth project block (or top-level working/delivered/sources compatibility)
    let proj_node = root.get("project").unwrap_or(&root);
    if let Some(working) = proj_node.get("working").and_then(yaml_string_owned) {
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "synth:workingDir",
            &working,
            namespace,
            lamport,
        );
    }
    if let Some(delivered) = proj_node.get("delivered").and_then(yaml_string_owned) {
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "synth:deliveredDir",
            &delivered,
            namespace,
            lamport,
        );
    }
    if let Some(YamlValue::Sequence(sources)) = proj_node.get("sources") {
        for src in sources {
            if let Some(s) = yaml_string_owned(src) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    "synth:hasSource",
                    &s,
                    namespace,
                    lamport,
                );
            }
        }
    }

    // 1:1 SI Metric Dimensions (`dimensions_si`)
    if let Some(YamlValue::Mapping(dims)) = root.get("dimensions_si") {
        for (k, v) in dims {
            if let (Some(k_str), Some(v_str)) = (k.as_str(), yaml_string_owned(v)) {
                let pred = format!("q42:dimension:{k_str}");
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    &pred,
                    &v_str,
                    namespace,
                    lamport,
                );
            }
        }
    }

    // Three mandatory delivery profiles (`film`, `open_world_game`, `cartoon`)
    if let Some(YamlValue::Sequence(profiles)) = root.get("profiles") {
        for (pi, prof) in profiles.iter().enumerate() {
            let prof_id = prof
                .get("id")
                .and_then(yaml_string)
                .map(|s| format!("{asset_id_surface}#profile-{s}"))
                .unwrap_or_else(|| format!("{asset_id_surface}#profile-{pi}"));
            let prof_subj = lex_insert(&mut lexicon, &prof_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                "q42:hasDeliveryProfile",
                &prof_id,
                namespace,
                lamport,
            );
            for (k, pred) in [
                ("id", "dct:identifier"),
                ("target", "q42:profileTarget"),
                ("resource", "q42:profileResource"),
                ("shading_model", "q42:shadingModel"),
            ] {
                if let Some(v) = prof.get(k).and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        prof_subj,
                        pred,
                        &v,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    // Components (`components` for made-world BOM, or `organs` for biosphere anatomy)
    for (list_key, part_pred) in [
        ("components", "made:hasComponent"),
        ("organs", "q42bio:hasOrgan"),
    ] {
        if let Some(YamlValue::Sequence(parts)) = root.get(list_key) {
            for (ci, part) in parts.iter().enumerate() {
                let part_id = part
                    .get("did")
                    .or_else(|| part.get("@id"))
                    .or_else(|| part.get("id"))
                    .and_then(yaml_string)
                    .map(|s| {
                        if s.contains(':') {
                            s.to_string()
                        } else {
                            format!("{asset_id_surface}#part-{s}")
                        }
                    })
                    .unwrap_or_else(|| format!("{asset_id_surface}#part-{ci}"));
                let part_subj = lex_insert(&mut lexicon, &part_id);
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    "dct:hasPart",
                    &part_id,
                    namespace,
                    lamport,
                );
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    part_pred,
                    &part_id,
                    namespace,
                    lamport,
                );
                for (k, pred) in [
                    ("id", "dct:identifier"),
                    ("did", "dct:identifier"),
                    ("class", "rdf:type"),
                    ("ontology_class", "rdf:type"),
                    ("label", "rdfs:label"),
                    ("material", "made:hasMaterial"),
                ] {
                    if let Some(v) = part.get(k).and_then(yaml_string_owned) {
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            part_subj,
                            pred,
                            &v,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
        }
    }

    // Functional Ports (`ports` — REQ-MADE-006)
    if let Some(YamlValue::Sequence(ports)) = root.get("ports") {
        for (pi, port) in ports.iter().enumerate() {
            let port_id = port
                .get("did")
                .or_else(|| port.get("id"))
                .and_then(yaml_string)
                .map(|s| {
                    if s.contains(':') {
                        s.to_string()
                    } else {
                        format!("{asset_id_surface}#port-{s}")
                    }
                })
                .unwrap_or_else(|| format!("{asset_id_surface}#port-{pi}"));
            let port_subj = lex_insert(&mut lexicon, &port_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                "made:hasPort",
                &port_id,
                namespace,
                lamport,
            );
            for (k, pred) in [
                ("id", "dct:identifier"),
                ("medium", "made:portMedium"),
                ("direction", "made:portDirection"),
                ("unit", "qudt:unit"),
                ("rated_capacity", "made:ratedCapacity"),
            ] {
                if let Some(v) = port.get(k).and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        port_subj,
                        pred,
                        &v,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    // InstanceState64 Schema (`instance_state_64`)
    if let Some(ist) = root.get("instance_state_64") {
        let schema_id = ist
            .get("id")
            .and_then(yaml_string)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{asset_id_surface}#instance-state-64"));
        let schema_subj = lex_insert(&mut lexicon, &schema_id);
        emit_spo(
            &mut quins,
            &mut lexicon,
            asset_subject,
            "q42state:hasInstanceStateSchema",
            &schema_id,
            namespace,
            lamport,
        );
        emit_spo(
            &mut quins,
            &mut lexicon,
            schema_subj,
            "rdf:type",
            "q42state:InstanceState64Schema",
            namespace,
            lamport,
        );
        for (k, pred) in [
            ("struct_size_bytes", "q42state:structSizeBytes"),
            ("axes", "q42state:environmentalStressAxes"),
        ] {
            if let Some(v) = ist.get(k).and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    schema_subj,
                    pred,
                    &v,
                    namespace,
                    lamport,
                );
            }
        }
        if let Some(YamlValue::Sequence(stages)) = ist.get("stages") {
            for (si, st) in stages.iter().enumerate() {
                let stage_id = st
                    .get("id")
                    .and_then(yaml_string)
                    .map(|s| format!("{asset_id_surface}#stage-{s}"))
                    .unwrap_or_else(|| format!("{asset_id_surface}#stage-{si}"));
                let stage_subj = lex_insert(&mut lexicon, &stage_id);
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    asset_subject,
                    "q42bio:hasDevelopmentalStage",
                    &stage_id,
                    namespace,
                    lamport,
                );
                if let Some(min_v) = st.get("min").and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        stage_subj,
                        "q42state:ageNormMin",
                        &min_v,
                        namespace,
                        lamport,
                    );
                }
                if let Some(max_v) = st.get("max").and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        stage_subj,
                        "q42state:ageNormMax",
                        &max_v,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    // DCAT Distributions (`distributions`)
    if let Some(YamlValue::Sequence(dists)) = root.get("distributions") {
        for (di, dist) in dists.iter().enumerate() {
            let dist_id = dist
                .get("id")
                .and_then(yaml_string)
                .map(|s| format!("{asset_id_surface}#dist-{s}"))
                .unwrap_or_else(|| format!("{asset_id_surface}#dist-{di}"));
            let dist_subj = lex_insert(&mut lexicon, &dist_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                "dcat:distribution",
                &dist_id,
                namespace,
                lamport,
            );
            emit_spo(
                &mut quins,
                &mut lexicon,
                dist_subj,
                "rdf:type",
                "dcat:Distribution",
                namespace,
                lamport,
            );
            for (k, pred) in [
                ("title", "dct:title"),
                ("format", "dct:format"),
                ("path", "dcat:accessURL"),
                ("access_url", "dcat:accessURL"),
                ("derived_from", "prov:wasDerivedFrom"),
                ("sha256", "q42:sha256"),
                ("bytes", "dcat:byteSize"),
                ("status", "q42:artifactStatus"),
                ("packet", "q42:productionPacket"),
            ] {
                if let Some(v) = dist.get(k).and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        dist_subj,
                        pred,
                        &v,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    // LOD Ladder (`lods`)
    if let Some(YamlValue::Sequence(lods)) = root.get("lods") {
        for (li, lod) in lods.iter().enumerate() {
            let lod_id = lod
                .get("id")
                .and_then(yaml_string)
                .map(|s| format!("{asset_id_surface}#lod-{s}"))
                .unwrap_or_else(|| format!("{asset_id_surface}#lod-{li}"));
            let lod_subj = lex_insert(&mut lexicon, &lod_id);
            emit_spo(
                &mut quins,
                &mut lexicon,
                asset_subject,
                "q42:hasLodLevel",
                &lod_id,
                namespace,
                lamport,
            );
            for (k, pred) in [
                ("id", "dct:identifier"),
                ("file", "dcat:accessURL"),
                ("distance_band_m", "q42:distanceBandMeters"),
                ("vertices", "q42:vertexCount"),
                ("triangles", "q42:triangleCount"),
                ("target_triangles", "q42:targetTriangles"),
                ("bytes", "dcat:byteSize"),
                ("sha256", "q42:sha256"),
            ] {
                if let Some(v) = lod.get(k).and_then(yaml_string_owned) {
                    emit_spo(
                        &mut quins,
                        &mut lexicon,
                        lod_subj,
                        pred,
                        &v,
                        namespace,
                        lamport,
                    );
                }
            }
        }
    }

    Ok((quins, lexicon))
}

/// Extract Synth project configuration from a `yaml-ld-q42/asset` file (`manifest.q42.yaml` or `synth.project.yaml`).
pub fn extract_synth_project_from_yaml_ld(
    yaml_bytes: &[u8],
) -> Result<YamlAssetProjectConfig, String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse yaml-ld-q42 project config: {e}"))?;

    let name = root
        .get("name")
        .or_else(|| root.get("slug"))
        .and_then(yaml_string_owned)
        .unwrap_or_default();
    let id = root
        .get("@id")
        .or_else(|| root.get("id"))
        .and_then(yaml_string_owned)
        .unwrap_or_else(|| format!("urn:qualia:asset:{name}"));
    let brief = root
        .get("brief")
        .or_else(|| root.get("title"))
        .and_then(yaml_string_owned)
        .unwrap_or_default();
    let domain = root
        .get("domain")
        .and_then(yaml_string_owned)
        .unwrap_or_else(|| "made_world".into());
    let fiction = root
        .get("fiction")
        .and_then(YamlValue::as_bool)
        .unwrap_or(false);
    let quality_profile = root
        .get("quality_profile")
        .and_then(yaml_string_owned)
        .unwrap_or_else(|| "production16".into());

    let proj = root.get("project").unwrap_or(&root);
    let working = proj
        .get("working")
        .and_then(yaml_string_owned)
        .unwrap_or_else(|| "ontology".into());
    let delivered = proj
        .get("delivered")
        .and_then(yaml_string_owned)
        .unwrap_or_else(|| "dist".into());
    let mut sources = Vec::new();
    if let Some(YamlValue::Sequence(seq)) = proj.get("sources") {
        for item in seq {
            if let Some(s) = yaml_string_owned(item) {
                sources.push(s);
            }
        }
    }

    Ok(YamlAssetProjectConfig {
        id,
        name,
        brief,
        domain,
        fiction,
        quality_profile,
        working,
        delivered,
        sources,
    })
}

/// Auto-dispatch yaml-ld-q42 compilation:
/// 1. Asset manifest (`@type: HypermediaAssetManifest` or `distributions:`/`lods:`/`components:`/`project:`)
///    → [`compile_asset_yaml_ld_to_quins`] (`"yaml-ld-q42/asset"`)
/// 2. Workspace `pages:` present → [`compile_workspace_yaml_ld_to_quins`] (`"yaml-ld-q42/workspace"`)
/// 3. HCF `HypermediaDocument` / `content:` → [`compile_hcf_yaml_ld_to_quins`] (`"yaml-ld-q42/hcf"`)
/// 4. Else error with a clear message.
pub fn compile_yaml_ld_q42_auto(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport: u64,
) -> Result<(Vec<NQuin>, HashMap<u64, String>, &'static str), String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse yaml-ld-q42: {e}"))?;

    if yaml_type_is_asset_manifest(&root) || yaml_has_asset_sections(&root) {
        let (quins, lexicon) = compile_asset_yaml_ld_to_quins(yaml_bytes, namespace, lamport)?;
        Ok((quins, lexicon, "yaml-ld-q42/asset"))
    } else if yaml_has_workspace_pages(&root) {
        let (quins, lexicon) =
            compile_workspace_yaml_ld_to_quins(yaml_bytes, namespace, lamport)
                .map_err(|e| e.to_string())?;
        Ok((quins, lexicon, "yaml-ld-q42/workspace"))
    } else if yaml_type_is_hypermedia_document(&root) || yaml_has_content_sections(&root) {
        let (quins, lexicon) = compile_hcf_yaml_ld_to_quins(yaml_bytes, namespace, lamport)?;
        Ok((quins, lexicon, "yaml-ld-q42/hcf"))
    } else {
        Err(
            "yaml-ld-q42: expected HypermediaAssetManifest (distributions:/lods:/components:/project:), \
             WebizenWorkspace (pages:), or HCF HypermediaDocument (@type HypermediaDocument or content: sections)"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKSPACE_YAML: &str = r##"
"@id": "did:q42:app:health-dashboard"
theme_tokens:
  primary: "#58a6ff"
pages:
  - url_path: "/health"
    name: "Health Dashboard"
    panes:
      - component_id: "sensor-data"
        x: 1
        y: 1
        w: 6
        h: 4
        data_bindings:
          - "did:q42:patient1#heartrate"
    views:
      - manifold: "did:q42:patient1#vitals"
        kind: "Scene3D"
        sensitivity: "RightsBounded"
        requires_attestation: true
"##;

    const HCF_YAML: &str = r##"
"@context":
  "@vocab": "https://qualiadb.org/schema/hcf#"
"@id": "doc:civics_writing_sample"
"@type": "HypermediaDocument"
"title": "Civics Writing Sample"
"author": "Timothy Holborn"
"date": "2026-09-26"
"licence": "PermissiveCommons"
"summary": "Minimal HCF for wasm ingest"
"depicts":
  - "did:q42:design:rover_field_utility_4wd"
"tags":
  - civics
  - yaml-ld-q42
"content":
  - "@type": "Section"
    "heading": "Introduction"
    "blocks":
      - "@type": "Paragraph"
        "text": "HypermediaDocument is the SoT; Markdown is an optional projector."
        "markdown": "**Bold** projector field"
"##;

    const ASSET_YAML: &str = r##"
---
"@context":
  "@vocab": "https://qualiadb.org/schema/q42#"
"@id": "urn:qualia:design:rover-field-utility-4wd:rev1"
"@type":
  - "HypermediaAssetManifest"
  - "made:Artefact"
name: "rover-field-utility-4wd"
title: "NPWS 4WD Field Utility Rover"
brief: "Made-World Hypermedia (.hmc) Digital Twin"
did: "did:q42:design:rover_field_utility_4wd"
domain: "made_world"
fiction: false
quality_profile: "production16"
licence_policy: "PermissiveCommons"
project:
  working: "ontology"
  delivered: "dist"
  sources:
    - "manifest.q42.yaml"
    - "ontology/artefact.ttl"
    - "source/rover-field-utility-4wd.vibe"
dimensions_si:
  length_m: 4.92
  curb_mass_kg: 2280.0
profiles:
  - id: "film"
    target: "linear_render"
    resource: "dist/rover-field-utility-4wd-pbr.10d"
  - id: "open_world_game"
    target: "realtime_streaming"
    resource: "dist/lods/rover-field-utility-4wd-lod0.10d"
  - id: "cartoon"
    target: "stylized_realtime"
    shading_model: "Stylized"
    resource: "dist/rover-field-utility-4wd.10d"
components:
  - id: "chassis-frame"
    did: "did:q42:design:rover_field_utility_4wd#part-chassis-frame"
    class: "made:StructuralFrame"
    label: "Ladder-frame steel chassis"
ports:
  - id: "aux-dc-bus-24v"
    medium: "electrical_dc"
    direction: "bidirectional"
    unit: "qudt:Watt"
    rated_capacity: 2400.0
instance_state_64:
  id: "q42state:rover_instance_state_64"
  struct_size_bytes: 64
  axes: "vec4<f32>: [steering, suspension, weathering, battery_soc]"
  stages:
    - id: "factory_new"
      min: 0.0
      max: 0.15
distributions:
  - id: "hmc-container"
    title: "Hypermedia Container"
    format: "application/vnd.qualia.hmc"
    path: "dist/rover-field-utility-4wd.hmc"
    derived_from: "source/rover-field-utility-4wd.vibe"
lods:
  - id: "lod0"
    file: "dist/lods/rover-field-utility-4wd-lod0.10d"
    distance_band_m: "0-25m"
    triangles: 65000
"##;

    #[test]
    fn compile_workspace_pages_emits_page_pane_and_manifold_view_quins() {
        let (quins, lex) =
            compile_workspace_yaml_ld_to_quins(WORKSPACE_YAML.as_bytes(), 42, 7).unwrap();
        assert!(
            quins.len() >= 8,
            "expected page + pane + view quins, got {}",
            quins.len()
        );
        let pred_page = q_hash("q42:SystemPageDef");
        let pred_pane = q_hash("q42:SystemPaneState");
        let pred_view = q_hash("q42:ManifoldViewDef");
        let pred_binding = q_hash("q42:paneDataBinding");
        assert!(quins.iter().any(|q| q.predicate == pred_page));
        assert!(quins.iter().any(|q| q.predicate == pred_pane));
        assert!(quins.iter().any(|q| q.predicate == pred_view));
        assert!(quins.iter().any(|q| q.predicate == pred_binding));
        assert!(lex.values().any(|s| s == "did:q42:patient1#vitals"));
        assert_eq!(quins[0].context, 42);
    }

    #[test]
    fn compile_hcf_emits_title_heading_text_and_lexicon() {
        let (quins, lexicon) = compile_hcf_yaml_ld_to_quins(HCF_YAML.as_bytes(), 1, 9).unwrap();
        assert!(!quins.is_empty());
        assert!(lexicon.values().any(|s| s == "Civics Writing Sample"));
        assert!(lexicon.values().any(|s| s == "Introduction"));
        assert!(lexicon
            .values()
            .any(|s| s.contains("HypermediaDocument is the SoT")));
        assert!(lexicon.values().any(|s| s.contains("Bold")));
        assert!(lexicon.values().any(|s| s == "civics"));
        assert!(lexicon
            .values()
            .any(|s| s == "did:q42:design:rover_field_utility_4wd"));
        let pred_title = q_hash("hcf:title");
        assert!(quins.iter().any(|q| q.predicate == pred_title));
        let pred_heading = q_hash("hcf:heading");
        assert!(quins.iter().any(|q| q.predicate == pred_heading));
        let pred_text = q_hash("hcf:text");
        assert!(quins.iter().any(|q| q.predicate == pred_text));
        let pred_md = q_hash("hcf:markdown");
        assert!(quins.iter().any(|q| q.predicate == pred_md));
    }

    #[test]
    fn compile_asset_manifest_emits_dcat_bom_ports_lods_and_extracts_project() {
        let (quins, lex, fmt) = compile_yaml_ld_q42_auto(ASSET_YAML.as_bytes(), 7, 1).unwrap();
        assert_eq!(fmt, "yaml-ld-q42/asset");
        assert!(quins.len() >= 25, "got {} quins", quins.len());
        assert!(lex
            .values()
            .any(|s| s == "did:q42:design:rover_field_utility_4wd#part-chassis-frame"));
        assert!(lex.values().any(|s| s == "dist/rover-field-utility-4wd.hmc"));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("made:hasComponent")));
        assert!(quins.iter().any(|q| q.predicate == q_hash("made:hasPort")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("dcat:distribution")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42:hasLodLevel")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42:hasDeliveryProfile")));

        let cfg = extract_synth_project_from_yaml_ld(ASSET_YAML.as_bytes()).unwrap();
        assert_eq!(cfg.name, "rover-field-utility-4wd");
        assert_eq!(cfg.domain, "made_world");
        assert_eq!(cfg.working, "ontology");
        assert_eq!(cfg.delivered, "dist");
        assert_eq!(cfg.sources.len(), 3);
    }

    #[test]
    fn auto_dispatches_workspace_hcf_and_asset() {
        let (wq, wlex, wfmt) = compile_yaml_ld_q42_auto(WORKSPACE_YAML.as_bytes(), 0, 1).unwrap();
        assert_eq!(wfmt, "yaml-ld-q42/workspace");
        assert!(wq.len() >= 2);
        assert!(!wlex.is_empty());

        let (hq, hlex, hfmt) = compile_yaml_ld_q42_auto(HCF_YAML.as_bytes(), 0, 1).unwrap();
        assert_eq!(hfmt, "yaml-ld-q42/hcf");
        assert!(!hq.is_empty());
        assert!(!hlex.is_empty());

        let (aq, alex, afmt) = compile_yaml_ld_q42_auto(ASSET_YAML.as_bytes(), 0, 1).unwrap();
        assert_eq!(afmt, "yaml-ld-q42/asset");
        assert!(!aq.is_empty());
        assert!(!alex.is_empty());
    }

    #[test]
    fn auto_rejects_unknown_shape() {
        let err = compile_yaml_ld_q42_auto(b"foo: bar\n", 0, 0).unwrap_err();
        assert!(
            err.contains("HypermediaAssetManifest")
                || err.contains("WebizenWorkspace")
                || err.contains("HypermediaDocument"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn hcf_content_without_type_still_compiles() {
        let yaml = r#"
"@id": "doc:bare"
"content":
  - heading: "Only section"
    blocks:
      - text: "hello"
"#;
        let (quins, lexicon) = compile_hcf_yaml_ld_to_quins(yaml.as_bytes(), 0, 0).unwrap();
        assert!(!quins.is_empty());
        assert!(lexicon.values().any(|s| s == "Only section"));
        assert!(lexicon.values().any(|s| s == "hello"));
    }

    #[test]
    fn compile_multiscale_computational_strata_and_archetype_instance_with_utf8_labels() {
        let yaml = r##"
"@id": "urn:qualia:instance:corymbia-tree-042"
"@type": "HypermediaAssetManifest"
name: "corymbia-aparrerinja-instance-042"
domain: "biosphere"
epistemic_zone: "reality"
embodiment_mode: "archetype_instance"
labels:
  en: "Ghost Gum"
  rer: "Ilwempe"
  zh: "幽灵桉"
  ar: "شجرة الصمغ الشبح"
coordinates_10d: [1.0, 3.0, 2.0, -23.6980, 133.8807, 582.4, 1732060800.0, 0.92, 0.99, 0.55]
archetype_ref:
  digest: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  digest_alg: "BLAKE3-256"
  entity_id: "did:q42:taxon:corymbia_aparrerinja#archetype"
  taxon_iri: "https://intenteco.org/taxa/Corymbia_aparrerinja"
  seed: 987654321012345678
  age_days: 14600.0
  stage: "Adult"
computational_strata:
  - id: "leaf-lamina"
    level: "L1_OrganOrSubsystem"
    kind: "OrganOrSubAssembly"
    embodiment_state: "ArchetypeReferenced"
    scale_m: 0.12
  - id: "leaf-trichomes-and-stomata"
    level: "L2_TissueOrFollicle"
    kind: "FollicleOrTrichome"
    parent: "leaf-lamina"
    embodiment_state: "ProceduralSynthesized"
    scale_m: 0.00015
  - id: "endophytic-mycelium-and-phyllosphere-bacteria"
    level: "L3_HarbouredConsortia"
    kind: "EndophyticOrDermatophyticFungus"
    regime: "MutualistSymbiont"
    parent: "leaf-lamina"
    taxon_iri: "https://intenteco.org/taxa/Botryosphaeria_australis"
    embodiment_state: "ComputationalOnly"
    n3_rule: "urn:qualia:logic:EndophyteDroughtResilienceRule"
    belnap_status: "True"
  - id: "chloroplast-photosystem-ii"
    level: "L4_CellularOrOrganellar"
    kind: "ChloroplastOrPlastid"
    parent: "leaf-lamina"
    embodiment_state: "ComputationalOnly"
  - id: "chlorophyll-a-reaction-center-dft"
    level: "L6_QuantumChemistry"
    kind: "DftExchangeCorrelationGrid"
    regime: "ElectronicExcitation"
    parent: "chloroplast-photosystem-ii"
    quantum_basis: "def2-SVP"
    xc_functional: "GGA-PBE-Autodiff"
    hartree_energy: -2934.12841797
    homo_lumo_gap_ev: 1.86
    qpu_nquin: "urn:qualia:qforge:statevector:ps2_chl_a_excited"
"##;
        let (quins, lex, fmt) = compile_yaml_ld_q42_auto(yaml.as_bytes(), 99, 12).unwrap();
        assert_eq!(fmt, "yaml-ld-q42/asset");
        assert!(lex.values().any(|s| s == "Ilwempe@rer"));
        assert!(lex.values().any(|s| s == "幽灵桉@zh"));
        assert!(lex.values().any(|s| s == "شجرة الصمغ الشبح@ar"));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42:hasArchetypeRef")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42:hasComputationalStratum")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42chem:hartreeEnergy")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42chem:qForgeStateVectorNQuin")));
        assert!(quins
            .iter()
            .any(|q| q.predicate == q_hash("q42:coord10d:mu")));
    }
}
