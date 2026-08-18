//! Poet / Vibe 0.1 harness commands (P8) + document gazetteer (P7).
//!
//! Two tracks: Vibe reaches existing Qualia capabilities for humans/apps.
//! Gazetteer is document NLP (`qualia_core_db::nlp`), not the language.
//! Honesty: graph is an in-process snapshot until daemon wiring (Partial).

use qualia_core_db::poet_host::catalog::{engine_families_mcp_only, VIBE_0_1};
use qualia_core_db::poet_host::{format_value, PoetSnapshot};
use qualia_core_db::text_span::{annotation_quin, TextSpan};
use qualia_core_db::nlp::analyze_document;
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

pub struct PoetHarnessState {
    pub(crate) snap: Mutex<PoetSnapshot>,
    pub(crate) cells: Mutex<Vec<CellEntry>>,
}

/// A registered reactive cell. The harness re-evaluates cells whose
/// `graph_revision_at_eval` is older than the current snapshot revision
/// when `poet_recompute` is called. Only cells that called `graph.query`
/// during evaluation are considered graph-dependent.
#[derive(Clone, serde::Serialize)]
pub struct CellEntry {
    pub source: String,
    pub value: String,
    pub ok: bool,
    pub graph_dependent: bool,
    pub graph_revision_at_eval: u64,
    pub diagnostic: Option<String>,
}

impl Default for PoetHarnessState {
    fn default() -> Self {
        Self {
            snap: Mutex::new(PoetSnapshot::live()),
            cells: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Serialize)]
pub struct PoetEvalResult {
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
    pub revision: u64,
    pub committed: usize,
    pub published: Vec<String>,
    pub honesty: &'static str,
    pub language: &'static str,
    pub value_cbor_hex: String,
}

#[derive(Serialize)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
    pub start_utf8: u32,
    pub end_utf8: u32,
}

#[derive(Serialize)]
pub struct PoetGazetteerResult {
    pub ok: bool,
    pub diagnostic: Option<String>,
    pub token_count: usize,
    pub sentence_count: usize,
    pub source_hash: String,
    pub hits: Vec<GazetteerHitDto>,
    pub sealed: usize,
    pub revision: u64,
    pub honesty: &'static str,
}

fn snapshot_result(snap: &PoetSnapshot, ok: bool, value: String, diagnostic: Option<String>) -> PoetEvalResult {
    PoetEvalResult {
        ok,
        value: value.clone(),
        diagnostic,
        revision: snap.revision,
        committed: snap.visible_count(),
        published: snap.published.clone(),
        honesty: snap.honesty(),
        language: poet_vibe::LANGUAGE_VERSION,
        value_cbor_hex: encode_cbor_text(&value),
    }
}

fn encode_cbor_text(s: &str) -> String {
    let mut out = Vec::new();
    // Major type 3 (text), definite length — CBOR diagnostic for the result string.
    let b = s.as_bytes();
    if b.len() < 24 {
        out.push(0x60 | b.len() as u8);
    } else if b.len() < 256 {
        out.push(0x78);
        out.push(b.len() as u8);
    } else {
        out.push(0x79);
        out.extend_from_slice(&(b.len() as u16).to_be_bytes());
    }
    out.extend_from_slice(b);
    out.iter().map(|x| format!("{x:02x}")).collect()
}

#[tauri::command]
pub fn poet_eval(
    state: State<PoetHarnessState>,
    source: String,
    as_cell: bool,
    function: Option<String>,
) -> PoetEvalResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let run = if as_cell {
        snap.eval_cell_src(&source)
    } else if let Some(name) = function {
        snap.eval_fn(&source, &name, Vec::new())
    } else {
        snap.eval_cell_src(&source)
    };
    let result = match run {
        Ok(v) => snapshot_result(&snap, true, format_value(&v), None),
        Err(e) => snapshot_result(&snap, false, String::new(), Some(e.to_json())),
    };
    // Register reactive cells for later recomputation.
    if as_cell {
        let graph_dep = snap.graph_read_during_eval;
        let entry = CellEntry {
            source: source.clone(),
            value: result.value.clone(),
            ok: result.ok,
            graph_dependent: graph_dep,
            graph_revision_at_eval: snap.revision,
            diagnostic: result.diagnostic.clone(),
        };
        let mut cells = state.cells.lock().expect("poet cells");
        // Replace existing cell with same source, or add new.
        if let Some(existing) = cells.iter_mut().find(|c| c.source == source) {
            *existing = entry;
        } else {
            cells.push(entry);
        }
    }
    result
}

#[tauri::command]
pub fn poet_reset(state: State<PoetHarnessState>) -> PoetEvalResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    *snap = PoetSnapshot::live();
    let mut cells = state.cells.lock().expect("poet cells");
    cells.clear();
    snapshot_result(&snap, true, "reset".into(), None)
}

/// Re-evaluate all registered reactive cells whose graph dependency is stale
/// (i.e. `graph_revision_at_eval` < current snapshot revision). Cells that
/// didn't query the graph are not recomputed. Returns the updated cell list.
#[tauri::command]
pub fn poet_recompute(state: State<PoetHarnessState>) -> Vec<CellEntry> {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let current_revision = snap.revision;
    let mut cells = state.cells.lock().expect("poet cells");

    for cell in cells.iter_mut() {
        // Only re-evaluate graph-dependent cells whose revision is stale.
        if !cell.graph_dependent || cell.graph_revision_at_eval >= current_revision {
            continue;
        }
        let run = snap.eval_cell_src(&cell.source);
        match run {
            Ok(v) => {
                cell.value = format_value(&v);
                cell.ok = true;
                cell.diagnostic = None;
                cell.graph_revision_at_eval = snap.revision;
                // Re-check graph dependency after recomputation.
                cell.graph_dependent = snap.graph_read_during_eval;
            }
            Err(e) => {
                cell.ok = false;
                cell.value = String::new();
                cell.diagnostic = Some(e.to_json());
                cell.graph_revision_at_eval = snap.revision;
            }
        }
    }
    cells.clone()
}

/// List all registered reactive cells with their current state.
#[tauri::command]
pub fn poet_cells(state: State<PoetHarnessState>) -> Vec<CellEntry> {
    let cells = state.cells.lock().expect("poet cells");
    cells.clone()
}

#[tauri::command]
pub fn poet_gazetteer(state: State<PoetHarnessState>, source: String) -> PoetGazetteerResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    if source.len() > 256 * 1024 {
        return PoetGazetteerResult {
            ok: false,
            diagnostic: Some("document exceeds 256 KiB budget".into()),
            token_count: 0,
            sentence_count: 0,
            source_hash: String::new(),
            hits: Vec::new(),
            sealed: 0,
            revision: snap.revision,
            honesty: snap.honesty(),
        };
    }
    let analysis = analyze_document(&source);
    let mut sealed = 0usize;
    let mut hits = Vec::new();
    for plan in &analysis.plans {
        hits.push(GazetteerHitDto {
            surface: plan.surface.clone(),
            iri: plan.term_iri.clone(),
            kind: plan.kind.to_string(),
            start_utf8: plan.start_utf8,
            end_utf8: plan.end_utf8,
        });
        if let Some(span) = TextSpan::from_source(&source, plan.start_utf8, plan.end_utf8) {
            snap.ingest_sealed(annotation_quin(&plan.term_iri, span, plan.source_hash));
            sealed += 1;
        }
    }
    if sealed > 0 {
        snap.bump_revision();
    }
    PoetGazetteerResult {
        ok: true,
        diagnostic: None,
        token_count: analysis.token_count,
        sentence_count: analysis.sentence_count,
        source_hash: format!("{:#x}", analysis.source_hash),
        hits,
        sealed,
        revision: snap.revision,
        honesty: snap.honesty(),
    }
}

#[derive(Serialize)]
pub struct CapabilityRow {
    pub id: String,
    pub family: String,
    pub honesty: String,
    pub required: bool,
    pub vibe_bound: bool,
    pub mcp_tools: Vec<String>,
    pub maturity: String,
}

#[derive(Serialize)]
pub struct PoetCatalog {
    pub language: &'static str,
    pub honesty: &'static str,
    pub vibe: Vec<CapabilityRow>,
    pub engine_not_yet_on_vibe: Vec<CapabilityRow>,
}

#[tauri::command]
pub fn poet_capabilities() -> PoetCatalog {
    let vibe = VIBE_0_1
        .iter()
        .map(|b| {
            let desc = qualia_core_db::CAPABILITY_DESCRIPTORS
                .iter()
                .find(|d| d.name == b.family);
            CapabilityRow {
                id: b.id.into(),
                family: b.family.into(),
                honesty: b.honesty.into(),
                required: b.required,
                vibe_bound: true,
                mcp_tools: desc
                    .map(|d| d.mcp_tools.iter().map(|t| (*t).to_string()).collect())
                    .unwrap_or_default(),
                maturity: desc.map(|d| d.maturity.to_string()).unwrap_or_default(),
            }
        })
        .collect();
    let engine_not_yet_on_vibe = engine_families_mcp_only()
        .into_iter()
        .map(|d| CapabilityRow {
            id: d.name.into(),
            family: d.domain.into(),
            honesty: "unbound".into(),
            required: false,
            vibe_bound: false,
            mcp_tools: d.mcp_tools.iter().map(|t| (*t).to_string()).collect(),
            maturity: d.maturity.into(),
        })
        .collect();
    PoetCatalog {
        language: poet_vibe::LANGUAGE_VERSION,
        honesty: "partial",
        vibe,
        engine_not_yet_on_vibe,
    }
}
