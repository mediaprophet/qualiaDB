//! WASM-bindgen API — dataio domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonLdFlatTriple {
    pub s: String,
    pub p: String,
    pub o: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_json_wasm(payload: &str) -> JsValue {
    if let Ok(triples) = serde_json::from_str::<Vec<JsonLdFlatTriple>>(payload) {
        #[derive(Serialize)]
        struct QOut {
            subject: String,
            predicate: String,
            object: String,
        }

        let mut out = Vec::new();
        for t in triples {
            out.push(QOut {
                subject: t.s,
                predicate: t.p,
                object: t.o,
            });
        }
        serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
    } else {
        JsValue::NULL
    }
}

// ─── LWW CRDT ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize, Serialize, Clone)]
pub struct QuinJson {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

// --- Data Format: CSV Parser -----------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvParseParams {
    pub csv_data: String,
    pub base_class_hash: u64,
    pub field_mappings: Vec<CsvFieldMapping>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: String, // "integer", "float", "datetime", "string"
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_csv_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::parsers::csv_parser::{
        parse_csv_to_quins, CsvColumnMapping, CsvDatatype, CsvMappingProfile,
    };
    use std::io::Cursor;

    let p: CsvParseParams = serde_wasm_bindgen::from_value(val)?;

    let mut profile = CsvMappingProfile {
        base_class_hash: p.base_class_hash,
        fields: p
            .field_mappings
            .iter()
            .map(|f| CsvColumnMapping {
                source_key: f.source_key.clone(),
                column_index: None,
                predicate_hash: f.predicate_hash,
                datatype: match f.datatype.as_str() {
                    "integer" => CsvDatatype::Integer,
                    "float" => CsvDatatype::Float,
                    "datetime" => CsvDatatype::DateTime,
                    _ => CsvDatatype::StringRef,
                },
            })
            .collect(),
    };

    let mut quins = Vec::new();
    let cursor = Cursor::new(p.csv_data.as_bytes());

    parse_csv_to_quins(cursor, &mut profile, |quin| {
        quins.push(quin);
    })
    .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct ParseResult {
        quin_count: usize,
        // JavaScript Numbers cannot faithfully represent every u64. Emit decimal
        // strings so callers retain the exact Quin payloads across the WASM ABI.
        quins: Vec<[String; 6]>,
    }

    let quin_arrays: Vec<[String; 6]> = quins
        .iter()
        .map(|q| {
            [
                q.subject.to_string(),
                q.predicate.to_string(),
                q.object.to_string(),
                q.context.to_string(),
                q.metadata.to_string(),
                q.parity.to_string(),
            ]
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&ParseResult {
        quin_count: quins.len(),
        quins: quin_arrays,
    })?)
}

// --- Data Format: JSON Parser ----------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonParseParams {
    pub json_data: String,
    pub base_class_hash: u64,
    pub field_mappings: Vec<JsonFieldMapping>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonFieldMapping {
    pub source_key: String,
    pub predicate_hash: u64,
    pub datatype: String, // "integer", "float", "datetime", "string"
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_json_mapping_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::parsers::json_parser::{
        parse_json_to_quins, JsonDatatype, JsonFieldMapping as CoreJsonFieldMapping,
        JsonMappingProfile,
    };
    use std::io::Cursor;

    let p: JsonParseParams = serde_wasm_bindgen::from_value(val)?;

    let profile = JsonMappingProfile {
        base_class_hash: p.base_class_hash,
        fields: p
            .field_mappings
            .iter()
            .map(|f| CoreJsonFieldMapping {
                source_key: f.source_key.clone(),
                predicate_hash: f.predicate_hash,
                datatype: match f.datatype.as_str() {
                    "integer" => JsonDatatype::Integer,
                    "float" => JsonDatatype::Float,
                    "datetime" => JsonDatatype::DateTime,
                    _ => JsonDatatype::StringRef,
                },
            })
            .collect(),
    };

    let mut quins = Vec::new();
    let cursor = Cursor::new(p.json_data.as_bytes());

    parse_json_to_quins(cursor, &profile, |quin| {
        quins.push(quin);
    })
    .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct ParseResult {
        quin_count: usize,
        quins: Vec<[String; 6]>,
    }

    let quin_arrays: Vec<[String; 6]> = quins
        .iter()
        .map(|q| {
            [
                q.subject.to_string(),
                q.predicate.to_string(),
                q.object.to_string(),
                q.context.to_string(),
                q.metadata.to_string(),
                q.parity.to_string(),
            ]
        })
        .collect();

    Ok(serde_wasm_bindgen::to_value(&ParseResult {
        quin_count: quins.len(),
        quins: quin_arrays,
    })?)
}

// --- Data Format: CSV Serializer -------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct CsvSerializeParams {
    pub quins: Vec<[u64; 6]>,
    pub field_names: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_csv_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::serialisers::csv_serializer::{
        serialize_quins_to_csv, CsvDatatype as CoreCsvDatatype, CsvSerializationProfile,
    };
    use crate::NQuin;

    let p: CsvSerializeParams = serde_wasm_bindgen::from_value(val)?;

    let quins: Vec<NQuin> = p
        .quins
        .iter()
        .map(|arr| NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect();

    let profile = CsvSerializationProfile {
        headers: p.field_names,
        predicate_hashes: p.predicate_hashes,
        datatypes: p
            .datatypes
            .iter()
            .map(|d| match d.as_str() {
                "integer" => CoreCsvDatatype::Integer,
                "float" => CoreCsvDatatype::Float,
                "datetime" => CoreCsvDatatype::DateTime,
                _ => CoreCsvDatatype::StringRef,
            })
            .collect(),
    };

    let mut csv_output = Vec::new();
    serialize_quins_to_csv(&mut csv_output, &quins, &profile).map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct SerializeResult {
        csv_data: String,
    }

    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        csv_data: String::from_utf8(csv_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
    })?)
}

// --- Data Format: JSON Serializer ------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonSerializeParams {
    pub quins: Vec<[u64; 6]>,
    pub field_names: Vec<String>,
    pub predicate_hashes: Vec<u64>,
    pub datatypes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_json_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::serialisers::json_serializer::{
        serialize_quins_to_json, JsonDatatype as CoreJsonDatatype, JsonSerializationProfile,
    };
    use crate::NQuin;

    let p: JsonSerializeParams = serde_wasm_bindgen::from_value(val)?;

    let quins: Vec<NQuin> = p
        .quins
        .iter()
        .map(|arr| NQuin {
            subject: arr[0],
            predicate: arr[1],
            object: arr[2],
            context: arr[3],
            metadata: arr[4],
            parity: arr[5],
        })
        .collect();

    let profile = JsonSerializationProfile {
        field_names: p.field_names,
        predicate_hashes: p.predicate_hashes,
        datatypes: p
            .datatypes
            .iter()
            .map(|d| match d.as_str() {
                "integer" => CoreJsonDatatype::Integer,
                "float" => CoreJsonDatatype::Float,
                "datetime" => CoreJsonDatatype::DateTime,
                _ => CoreJsonDatatype::StringRef,
            })
            .collect(),
    };

    let mut json_output = Vec::new();
    serialize_quins_to_json(&mut json_output, &quins, &profile)
        .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct SerializeResult {
        json_data: String,
    }

    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        json_data: String::from_utf8(json_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
    })?)
}

/// Parse and compile a yaml-ld-q42 document (workspace pages or HCF HypermediaDocument) into quins and lexicon.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_yaml_ld_q42_wasm(
    source: &str,
    namespace: Option<u64>,
    lamport: Option<u64>,
) -> Result<JsValue, JsValue> {
    let (quins, lexicon, variant) = crate::yaml_ld_q42::compile_yaml_ld_q42_auto(
        source.as_bytes(),
        namespace.unwrap_or(0),
        lamport.unwrap_or(0),
    )
    .map_err(|e| JsValue::from_str(&e))?;

    #[derive(Serialize)]
    struct YamlLdResult {
        variant: String,
        quin_count: usize,
        lexicon_count: usize,
        quins: Vec<[u64; 6]>,
        lexicon: Vec<(String, String)>,
    }

    let quins_out: Vec<[u64; 6]> = quins
        .iter()
        .map(|q| [q.subject, q.predicate, q.object, q.context, q.metadata, q.parity])
        .collect();

    let lexicon_out: Vec<(String, String)> = lexicon
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    Ok(serde_wasm_bindgen::to_value(&YamlLdResult {
        variant: variant.to_string(),
        quin_count: quins_out.len(),
        lexicon_count: lexicon_out.len(),
        quins: quins_out,
        lexicon: lexicon_out,
    })?)
}

// ─── Canonical HMC Bundle (QBDL) WASM Bridge (QG-01) ───────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct HmcEntryInfo {
    pub key: String,
    pub kind: String,
    pub offset: u64,
    pub length: u64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn list_hmc_bundle_entries_wasm(bundle_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let reader = crate::bundle::BundleReader::parse(bundle_bytes)
        .map_err(|e| JsValue::from_str(&format!("HMC parse error: {e}")))?;
    let entries: Vec<HmcEntryInfo> = reader
        .entries()
        .iter()
        .map(|e| HmcEntryInfo {
            key: e.key.clone(),
            kind: e.kind.clone(),
            offset: e.offset,
            length: e.length,
        })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&entries)?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn read_hmc_bundle_entry_wasm(bundle_bytes: &[u8], key: &str) -> Result<Vec<u8>, JsValue> {
    let reader = crate::bundle::BundleReader::parse(bundle_bytes)
        .map_err(|e| JsValue::from_str(&format!("HMC parse error: {e}")))?;
    let slice = reader
        .get(key)
        .ok_or_else(|| JsValue::from_str(&format!("HMC entry not found: {key}")))?;
    if !reader.verify_entry(key) {
        return Err(JsValue::from_str(&format!("HMC entry checksum mismatch: {key}")));
    }
    Ok(slice.to_vec())
}


// ─── Mutable Q42 Session WASM Bridge (QG-15) ────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmQ42Session {
    inner: crate::q42::journal::MutableQ42Session<std::io::Cursor<Vec<u8>>>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmQ42Session {
    #[wasm_bindgen(constructor)]
    pub fn new(world_did_hash: u64, base_digest_bytes: &[u8]) -> Result<WasmQ42Session, JsValue> {
        if base_digest_bytes.len() < 32 {
            return Err(JsValue::from_str("base_digest must be at least 32 bytes"));
        }
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&base_digest_bytes[0..32]);
        let header = crate::q42::journal::JournalHeader::new(
            crate::q42::journal::JournalApplicationProfile::GenericSemantic,
            world_did_hash,
            digest,
            1,
        );
        let storage = std::io::Cursor::new(Vec::new());
        let journal = crate::q42::journal::Q42Journal::create(storage, header)
            .map_err(|e| JsValue::from_str(&format!("Journal create error: {e}")))?;
        let session = crate::q42::journal::MutableQ42Session::open(journal, &[])
            .map_err(|e| JsValue::from_str(&format!("Session open error: {e}")))?;
        Ok(WasmQ42Session { inner: session })
    }

    /// Load an existing durable Q42 session from journal bytes and optional base Quin snapshot bytes.
    pub fn load_from_bytes(
        journal_bytes: &[u8],
        base_quins_bytes: &[u8],
    ) -> Result<WasmQ42Session, JsValue> {
        let mut base_quins = Vec::new();
        if !base_quins_bytes.is_empty() {
            for chunk in base_quins_bytes.chunks_exact(48) {
                let q = crate::q42::journal::quin_from_bytes(chunk)
                    .map_err(|e| JsValue::from_str(&format!("Invalid base quin: {e}")))?;
                base_quins.push(q);
            }
        }
        let storage = std::io::Cursor::new(journal_bytes.to_vec());
        let journal = crate::q42::journal::Q42Journal::open_any(storage)
            .map_err(|e| JsValue::from_str(&format!("Failed to open journal: {e}")))?;
        let session = crate::q42::journal::MutableQ42Session::open(journal, &base_quins)
            .map_err(|e| JsValue::from_str(&format!("Failed to open session: {e}")))?;
        Ok(WasmQ42Session { inner: session })
    }

    pub fn stage_add_quin(&mut self, s: u64, p: u64, o: u64, c: u64, m: u64) {
        let parity = s ^ p ^ o ^ c ^ m;
        self.inner.stage_add(crate::NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity,
        });
    }

    pub fn stage_remove_quin(&mut self, s: u64, p: u64, o: u64, c: u64, m: u64) {
        let parity = s ^ p ^ o ^ c ^ m;
        self.inner.stage_remove(crate::NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity,
        });
    }

    pub fn rollback_staged(&mut self) {
        self.inner.rollback_staged();
    }

    pub fn commit_transaction(&mut self, command_hash: u64, actor_did: u64) -> Result<u32, JsValue> {
        self.inner
            .commit_transaction(command_hash, actor_did)
            .map_err(|e| JsValue::from_str(&format!("Commit error: {e}")))
    }

    pub fn active_quin_count(&self) -> usize {
        self.inner.active_state.len()
    }

    pub fn export_journal_bytes(&self) -> Vec<u8> {
        self.inner.journal.storage.get_ref().clone()
    }

    /// Export all currently active Quins as a flat contiguous 48-byte buffer.
    pub fn export_active_quins_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.inner.active_state.len() * 48);
        for q in &self.inner.active_state {
            bytes.extend_from_slice(&crate::q42::journal::quin_to_bytes(q));
        }
        bytes
    }

    /// Check if a Quin matching the given pattern exists in the active session (0 = wildcard).
    pub fn query_has_quin(&self, s: u64, p: u64, o: u64) -> bool {
        for q in &self.inner.active_state {
            if (s == 0 || q.subject == s)
                && (p == 0 || q.predicate == p)
                && (o == 0 || q.object == o)
            {
                return true;
            }
        }
        false
    }

    /// Query the object value for a given (subject, predicate) pair.
    pub fn query_object_for_predicate(&self, s: u64, p: u64) -> Option<u64> {
        for q in &self.inner.active_state {
            if q.subject == s && q.predicate == p {
                return Some(q.object);
            }
        }
        None
    }

    /// Rewind session state to a specific simulation/historical tick.
    pub fn rewind_to_tick(&mut self, base_quins_bytes: &[u8], target_tick: u64) -> Result<(), JsValue> {
        let mut base_quins = Vec::new();
        if !base_quins_bytes.is_empty() {
            for chunk in base_quins_bytes.chunks_exact(48) {
                let q = crate::q42::journal::quin_from_bytes(chunk)
                    .map_err(|e| JsValue::from_str(&format!("Invalid base quin: {e}")))?;
                base_quins.push(q);
            }
        }
        self.inner
            .rewind_to_tick(&base_quins, target_tick)
            .map_err(|e| JsValue::from_str(&format!("Rewind error: {e}")))
    }

    pub fn current_tick(&self) -> u64 {
        self.inner.current_tick()
    }

    pub fn advance_tick(&mut self, tick: u64) {
        self.inner.advance_tick(tick);
    }
}

// ─── Fixed-Tick Simulation Engine WASM Bridge (QG-09) ───────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmSimulationWorld {
    inner: crate::simulation::fixed_tick::FixedTickWorld<64, 256>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmSimulationWorld {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> WasmSimulationWorld {
        WasmSimulationWorld {
            inner: crate::simulation::fixed_tick::FixedTickWorld::new(seed),
        }
    }

    pub fn register_agent(&mut self, entity_id: u64, x_mm: i64, y_mm: i64) -> bool {
        self.inner.register_agent(entity_id, x_mm, y_mm)
    }

    pub fn submit_command(
        &mut self,
        target_tick: u64,
        sequence: u32,
        actor_did: u64,
        command_opcode: u16,
        target_entity: u64,
        arg0: i64,
        arg1: i64,
    ) -> Result<(), JsValue> {
        let cmd = crate::simulation::fixed_tick::SimulationCommand {
            target_tick,
            sequence,
            actor_did,
            command_opcode,
            target_entity,
            arg0,
            arg1,
        };
        self.inner
            .submit_command(cmd)
            .map_err(|e| JsValue::from_str(&format!("Command rejected: {e:?}")))
    }

    pub fn step_tick(&mut self) -> usize {
        let mut receipts = [crate::simulation::fixed_tick::CommandReceipt::Rejected {
            tick: 0,
            command_hash: 0,
            reason: crate::simulation::fixed_tick::RejectionReason::InvalidTick,
        }; 16];
        self.inner.step_tick(&mut receipts)
    }

    pub fn compute_state_hash(&self) -> u64 {
        self.inner.compute_state_hash()
    }

    pub fn current_tick(&self) -> u64 {
        self.inner.current_tick
    }

    pub fn get_agent_pos_x_mm(&self, entity_id: u64) -> Option<i64> {
        self.inner.get_agent_position(entity_id).map(|(x, _)| x)
    }

    pub fn get_agent_pos_y_mm(&self, entity_id: u64) -> Option<i64> {
        self.inner.get_agent_position(entity_id).map(|(_, y)| y)
    }

    /// Spatial box query returning matching entity IDs as strings (for u64 JS precision).
    pub fn query_agents_in_bounds(
        &self,
        min_x_mm: i64,
        min_y_mm: i64,
        max_x_mm: i64,
        max_y_mm: i64,
    ) -> Vec<String> {
        let mut out_ids = [0u64; 64];
        let count = self.inner.query_agents_in_bounds(
            min_x_mm,
            min_y_mm,
            max_x_mm,
            max_y_mm,
            &mut out_ids,
        );
        out_ids[..count].iter().map(|id| id.to_string()).collect()
    }

    /// Deterministic 2D grid pathfinder (QG-11).
    /// Returns interleaved waypoint coordinates: [x0, y0, x1, y1, ...].
    pub fn find_path_grid(
        &self,
        start_x: i32,
        start_y: i32,
        target_x: i32,
        target_y: i32,
        obstacles: &[u8],
        width: usize,
        height: usize,
    ) -> Vec<i32> {
        let mut nav = crate::simulation::navigation::NavigationGrid::<128, 128>::new();
        nav.width = width.min(128);
        nav.height = height.min(128);
        let copy_len = obstacles.len().min(nav.width * nav.height);
        nav.cells[..copy_len].copy_from_slice(&obstacles[..copy_len]);

        let mut out_path = [(0i32, 0i32); 128];
        let len = nav.find_path((start_x, start_y), (target_x, target_y), &mut out_path);
        let mut flat = Vec::with_capacity(len * 2);
        for i in 0..len {
            flat.push(out_path[i].0);
            flat.push(out_path[i].1);
        }
        flat
    }

    /// Compute group formation destinations for `count` agents (QG-11).
    /// Returns interleaved destination coordinates: [x0, y0, x1, y1, ...].
    pub fn compute_group_formation(
        &self,
        target_x: i32,
        target_y: i32,
        count: usize,
        obstacles: &[u8],
        width: usize,
        height: usize,
    ) -> Vec<i32> {
        let mut nav = crate::simulation::navigation::NavigationGrid::<128, 128>::new();
        nav.width = width.min(128);
        nav.height = height.min(128);
        let copy_len = obstacles.len().min(nav.width * nav.height);
        nav.cells[..copy_len].copy_from_slice(&obstacles[..copy_len]);

        let mut out_dests = [(0i32, 0i32); 64];
        let written = nav.compute_group_formation_destinations(
            target_x,
            target_y,
            count.min(64),
            &mut out_dests,
        );
        let mut flat = Vec::with_capacity(written * 2);
        for i in 0..written {
            flat.push(out_dests[i].0);
            flat.push(out_dests[i].1);
        }
        flat
    }
}


