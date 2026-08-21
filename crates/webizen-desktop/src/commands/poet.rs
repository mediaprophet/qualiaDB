//! Poet / Vibe 0.1 harness commands (P8) + document gazetteer (P7).
//!
//! Two tracks: Vibe reaches existing Qualia capabilities for humans/apps.
//! Gazetteer is document NLP (`qualia_core_db::nlp`), not the language.
//! Honesty: graph is an in-process snapshot until daemon wiring (Partial).

use qualia_core_db::nlp::analyze_document;
use qualia_core_db::poet_host::catalog::{engine_families_mcp_only, VIBE_0_1};
use qualia_core_db::poet_host::{format_value, PoetSnapshot, PulseRecord};
use qualia_core_db::text_span::{annotation_quin, TextSpan};
use serde::Serialize;
use std::sync::Mutex;
use tauri::State;

pub struct PoetHarnessState {
    pub(crate) snap: Mutex<PoetSnapshot>,
    pub(crate) cells: Mutex<Vec<CellEntry>>,
    /// Stored program sources for hook dispatch (tick, pulse).
    /// Keyed by a caller-provided name.
    pub(crate) programs: Mutex<Vec<StoredProgram>>,
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
    /// True if the cell called `time.unix` during evaluation.
    /// Time-dependent cells are recomputed on `poet_tick`.
    pub time_dependent: bool,
}

/// A stored Vibe program for reactive hook dispatch.
#[derive(Clone, serde::Serialize)]
pub struct StoredProgram {
    pub name: String,
    pub source: String,
}

impl Default for PoetHarnessState {
    fn default() -> Self {
        Self {
            snap: Mutex::new(PoetSnapshot::live()),
            cells: Mutex::new(Vec::new()),
            programs: Mutex::new(Vec::new()),
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize)]
pub struct PulseRecordDto {
    pub topic: String,
    pub payload_summary: String,
    pub seq: u64,
}

impl From<&PulseRecord> for PulseRecordDto {
    fn from(r: &PulseRecord) -> Self {
        Self {
            topic: r.topic.clone(),
            payload_summary: r.payload_summary.clone(),
            seq: r.seq,
        }
    }
}

#[allow(dead_code)]
#[derive(Serialize)]
pub struct PoetEvalResult {
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
    pub revision: u64,
    pub committed: usize,
    pub published: Vec<PulseRecordDto>,
    pub honesty: &'static str,
    pub language: &'static str,
    pub value_cbor_hex: String,
}

#[allow(dead_code)]
#[derive(Serialize)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
    pub start_utf8: u32,
    pub end_utf8: u32,
}

#[allow(dead_code)]
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

fn snapshot_result(
    snap: &PoetSnapshot,
    ok: bool,
    value: String,
    diagnostic: Option<String>,
) -> PoetEvalResult {
    PoetEvalResult {
        ok,
        value: value.clone(),
        diagnostic,
        revision: snap.revision,
        committed: snap.visible_count(),
        published: snap.published.iter().map(PulseRecordDto::from).collect(),
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
        let time_dep = snap.time_read_during_eval;
        let entry = CellEntry {
            source: source.clone(),
            value: result.value.clone(),
            ok: result.ok,
            graph_dependent: graph_dep,
            graph_revision_at_eval: snap.revision,
            diagnostic: result.diagnostic.clone(),
            time_dependent: time_dep,
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
                cell.time_dependent = snap.time_read_during_eval;
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

/// Dispatch a hook event on a loaded program.
///
/// `path` is the event path segments (e.g. `["pulse", "message"]` for
/// `on pulse:message(…)`, `["tick"]` for `on tick(…)`). `args_json` is a
/// JSON array of argument values, parsed into Vibe `Value`s.
#[tauri::command]
pub fn poet_dispatch_hook(
    state: State<PoetHarnessState>,
    source: String,
    path: Vec<String>,
    args_json: String,
) -> PoetEvalResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let args = parse_hook_args(&args_json);
    let run = snap.dispatch_hook_src(&source, &path, args);
    match run {
        Ok(v) => snapshot_result(&snap, true, format_value(&v), None),
        Err(e) => snapshot_result(&snap, false, String::new(), Some(e.to_json())),
    }
}

/// Parse a JSON array of hook arguments into Vibe `Value`s.
/// Supports numbers, strings, booleans, and null. Unknown JSON types
/// fall back to `Value::Null`.
fn parse_hook_args(json: &str) -> Vec<poet_vibe::Value> {
    if json.trim().is_empty() || json.trim() == "[]" {
        return Vec::new();
    }
    let parsed: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    parsed.into_iter().map(json_to_vibe).collect()
}

fn json_to_vibe(v: serde_json::Value) -> poet_vibe::Value {
    match v {
        serde_json::Value::Null => poet_vibe::Value::Null,
        serde_json::Value::Bool(b) => poet_vibe::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                poet_vibe::Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                poet_vibe::Value::U64(u)
            } else {
                poet_vibe::Value::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => poet_vibe::Value::String(s),
        _ => poet_vibe::Value::Null,
    }
}

/// Store a Vibe program source for reactive hook dispatch (tick, pulse).
/// Returns the stored program list.
#[tauri::command]
pub fn poet_store_program(
    state: State<PoetHarnessState>,
    name: String,
    source: String,
) -> Vec<StoredProgram> {
    let mut programs = state.programs.lock().expect("poet programs");
    if let Some(existing) = programs.iter_mut().find(|p| p.name == name) {
        existing.source = source;
    } else {
        programs.push(StoredProgram { name, source });
    }
    programs.clone()
}

/// List stored programs.
#[tauri::command]
pub fn poet_programs(state: State<PoetHarnessState>) -> Vec<StoredProgram> {
    state.programs.lock().expect("poet programs").clone()
}

/// Dispatch `on tick()` hooks on all stored programs, then recompute
/// time-dependent cells. Returns the tick result and updated cell list.
#[derive(Serialize)]
pub struct TickResult {
    pub hooks_dispatched: u64,
    pub hook_results: Vec<TickHookResult>,
    pub cells: Vec<CellEntry>,
    pub published: Vec<PulseRecordDto>,
    pub revision: u64,
}

#[derive(Serialize)]
pub struct TickHookResult {
    pub program: String,
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
}

#[tauri::command]
pub fn poet_tick(state: State<PoetHarnessState>) -> TickResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let programs = state.programs.lock().expect("poet programs").clone();
    let mut hook_results = Vec::new();
    let mut hooks_dispatched = 0u64;
    let tick_path = vec!["tick".to_string()];

    for prog in &programs {
        let run = snap.dispatch_hook_src(&prog.source, &tick_path, vec![]);
        hooks_dispatched += 1;
        match run {
            Ok(v) => hook_results.push(TickHookResult {
                program: prog.name.clone(),
                ok: true,
                value: format_value(&v),
                diagnostic: None,
            }),
            Err(e) => hook_results.push(TickHookResult {
                program: prog.name.clone(),
                ok: false,
                value: String::new(),
                diagnostic: Some(e.to_json()),
            }),
        }
    }

    // Recompute time-dependent cells.
    let mut cells = state.cells.lock().expect("poet cells");
    for cell in cells.iter_mut() {
        if !cell.time_dependent {
            continue;
        }
        let run = snap.eval_cell_src(&cell.source);
        match run {
            Ok(v) => {
                cell.value = format_value(&v);
                cell.ok = true;
                cell.diagnostic = None;
                cell.graph_revision_at_eval = snap.revision;
                cell.graph_dependent = snap.graph_read_during_eval;
                cell.time_dependent = snap.time_read_during_eval;
            }
            Err(e) => {
                cell.ok = false;
                cell.value = String::new();
                cell.diagnostic = Some(e.to_json());
                cell.graph_revision_at_eval = snap.revision;
            }
        }
    }

    let published = snap.published.iter().map(PulseRecordDto::from).collect();
    let revision = snap.revision;
    drop(snap);
    let cells_clone = cells.clone();
    drop(cells);

    TickResult {
        hooks_dispatched,
        hook_results,
        cells: cells_clone,
        published,
        revision,
    }
}

/// Inject a pulse:message event into the harness, dispatching `on pulse:message`
/// hooks on all stored programs. Returns the dispatch results and any pulse
/// records produced.
#[derive(Serialize)]
pub struct PulseEventResult {
    pub hooks_dispatched: u64,
    pub hook_results: Vec<TickHookResult>,
    pub published: Vec<PulseRecordDto>,
    pub revision: u64,
}

#[tauri::command]
pub fn poet_pulse_event(
    state: State<PoetHarnessState>,
    topic: String,
    payload_json: String,
) -> PulseEventResult {
    let mut snap = state.snap.lock().expect("poet snapshot");
    let programs = state.programs.lock().expect("poet programs").clone();
    let mut hook_results = Vec::new();
    let mut hooks_dispatched = 0u64;
    let pulse_path = vec!["pulse".to_string(), "message".to_string()];

    // Parse payload from JSON.
    let payload = if payload_json.trim().is_empty() {
        poet_vibe::Value::Null
    } else {
        json_to_vibe(serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Null))
    };

    let args = vec![poet_vibe::Value::String(topic.clone()), payload];

    for prog in &programs {
        let run = snap.dispatch_hook_src(&prog.source, &pulse_path, args.clone());
        hooks_dispatched += 1;
        match run {
            Ok(v) => hook_results.push(TickHookResult {
                program: prog.name.clone(),
                ok: true,
                value: format_value(&v),
                diagnostic: None,
            }),
            Err(e) => hook_results.push(TickHookResult {
                program: prog.name.clone(),
                ok: false,
                value: String::new(),
                diagnostic: Some(e.to_json()),
            }),
        }
    }

    let published = snap.published.iter().map(PulseRecordDto::from).collect();
    let revision = snap.revision;

    PulseEventResult {
        hooks_dispatched,
        hook_results,
        published,
        revision,
    }
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
pub fn poet_capabilities(state: State<PoetHarnessState>) -> PoetCatalog {
    let snap = state.snap.lock().expect("poet snapshot");
    let attached = snap.attached;
    let overall_honesty = snap.honesty();
    let vibe = VIBE_0_1
        .iter()
        .map(|b| {
            let desc = qualia_core_db::CAPABILITY_DESCRIPTORS
                .iter()
                .find(|d| d.name == b.family);
            CapabilityRow {
                id: b.id.into(),
                family: b.family.into(),
                honesty: qualia_core_db::poet_host::catalog::dynamic_honesty(b.id, attached).into(),
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
        honesty: overall_honesty,
        vibe,
        engine_not_yet_on_vibe,
    }
}
