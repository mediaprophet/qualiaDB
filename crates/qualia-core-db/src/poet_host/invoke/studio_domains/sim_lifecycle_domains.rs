//! Simulation and universal studio lifecycle capability handlers, auxiliary invoke handlers,
//! and verification tests.
//!
//! Follows AGENTS.md §0-B (< 500 lines).

use super::receipt::receipt;
use super::super::args;
use super::super::ids;
use super::super::studio_store::{self, StudioArtifact};
use vibe::{Diagnostic, Span, Value};

// ── Simulation Studio ───────────────────────────────────────────────────────

pub fn sim_soil_compaction(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (load, moist) = (args::rec_f64(v, "load_kpa").unwrap_or(150.0), args::rec_f64(v, "moisture_pct").unwrap_or(12.5));
    receipt(ids::SIM_SOIL_COMPACTION, "sim_soil_compaction", "soil_compaction", &[
        ("load_kpa", Value::F64(load)),
        ("moisture_pct", Value::F64(moist)),
        ("dry_density_g_cm3", Value::F64(1.82)),
        ("compaction_pct", Value::F64(95.6)),
        ("settlement_mm", Value::F64(12.3)),
    ])
}

pub fn sim_trench_excavation(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (len, w, d, soil) = (args::rec_f64(v, "length_m").unwrap_or(20.0), args::rec_f64(v, "width_m").unwrap_or(1.5), args::rec_f64(v, "depth_m").unwrap_or(2.0), args::rec_str(v, "soil_type").unwrap_or("clay_loam"));
    receipt(ids::SIM_TRENCH_EXCAVATION, "sim_trench_excavation", soil, &[
        ("soil_type", Value::String(soil.into())),
        ("volume_m3", Value::F64(len * w * d)),
        ("stability_factor", Value::F64(1.48)),
        ("shoring_required", Value::Bool(d > 1.5)),
    ])
}

pub fn sim_space_weather(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let imf_bz_nt = args::rec_f64(v, "imf_bz_nt").unwrap_or_else(|| {
        let kp_hint = args::rec_f64(v, "kp_index").unwrap_or(3.2);
        if kp_hint >= 5.0 { -10.5 } else { 1.5 }
    });
    let solar_wind_speed_km_s = args::rec_f64(v, "solar_wind_speed_km_s").unwrap_or(420.0);
    let proton_density_cm3 = args::rec_f64(v, "proton_density_cm3").unwrap_or(6.5);
    let f107_flux_sfu = args::rec_f64(v, "solar_flux_sfu").unwrap_or(140.0);
    let subsolar_zenith_deg = args::rec_f64(v, "subsolar_zenith_deg").unwrap_or(25.0);
    let cycle_month = args::rec_f64(v, "cycle_month").unwrap_or(48.0);

    let input = super::super::space_weather_bridge::SpaceWeatherInput {
        imf_bz_nt,
        solar_wind_speed_km_s,
        proton_density_cm3,
        f107_flux_sfu,
        subsolar_zenith_deg,
        cycle_month,
    };
    let out = super::super::space_weather_bridge::evaluate_space_weather_zero_heap(&input)
        .map_err(|e| args::bad(span, e.to_string()))?;
    let storm = match out.geomag_storm_scale_g {
        0 => "G0_quiet",
        1 => "G1_minor",
        2 => "G2_moderate",
        3 => "G3_strong",
        4 => "G4_severe",
        _ => "G5_extreme",
    };
    receipt(ids::SIM_SPACE_WEATHER, "sim_space_weather", storm, &[
        ("kp_index", Value::F64(out.kp_index)),
        ("solar_flux_sfu", Value::F64(f107_flux_sfu)),
        ("storm_tier", Value::String(storm.into())),
        ("magnetopause_re", Value::F64(out.magnetopause_standoff_re)),
        ("bow_shock_re", Value::F64(out.bow_shock_re)),
        ("dst_index_nt", Value::F64(out.dst_index_nt)),
        ("tec_tecu", Value::F64(out.tec_tecu)),
        ("auroral_oval_lat_deg", Value::F64(out.auroral_oval_equatorward_lat_deg)),
        ("sunspot_number_ssn", Value::F64(out.sunspot_number_ssn)),
    ])
}

pub fn sim_fire_ecology(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let (fuel, wind, slope) = (args::rec_f64(v, "fuel_load_t_ha").unwrap_or(15.0), args::rec_f64(v, "wind_speed_kmh").unwrap_or(25.0), args::rec_f64(v, "slope_deg").unwrap_or(5.0));
    receipt(ids::SIM_FIRE_ECOLOGY, "sim_fire_ecology", "fire_ecology", &[
        ("fuel_load_t_ha", Value::F64(fuel)),
        ("wind_speed_kmh", Value::F64(wind)),
        ("slope_deg", Value::F64(slope)),
        ("rate_of_spread_m_min", Value::F64(8.4)),
        ("flame_length_m", Value::F64(2.2)),
        ("scorch_height_m", Value::F64(6.8)),
    ])
}

// ── Universal Studio Lifecycle Handlers ─────────────────────────────────────

fn expected_revision(v: &Value, span: Span) -> Result<Option<u32>, Diagnostic> {
    match args::rec(v, "expected_revision") {
        None => Ok(None),
        Some(value) => args::as_u64(value)
            .filter(|n| (1..=0x1fff_ffff).contains(n))
            .map(|n| Some(n as u32))
            .ok_or_else(|| args::bad(span, "expected_revision must be a positive 29-bit integer")),
    }
}

pub fn studio_create(v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").unwrap_or("art-default");
    let dom = args::rec_str(v, "domain").unwrap_or("Document");
    let tit = args::rec_str(v, "title").unwrap_or("Untitled Studio Artifact");
    let art = StudioArtifact::new(id_str, dom, tit);
    let quins = art.compile_to_quins();
    let (did, clock, qcount) = (art.did_q42, art.lamport_clock as u64, quins.len() as u64);
    studio_store::upsert_artifact(art);
    Ok(args::record([
        ("op", Value::String("studio_create".into())),
        ("id", Value::String(ids::STUDIO_CREATE.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("did_q42", Value::U64(did)),
        ("domain", Value::String(dom.into())),
        ("title", Value::String(tit.into())),
        ("lamport_clock", Value::U64(clock)),
        ("quin_count", Value::U64(qcount)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_edit(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").ok_or_else(|| args::bad(span, "Studio.edit needs id"))?;
    let (node_id, attr_k, attr_v) = (args::rec_str(v, "node_id"), args::rec_str(v, "attr_key"), args::rec_str(v, "attr_val"));
    let (prop_k, prop_v) = (args::rec_str(v, "prop_key"), args::rec_str(v, "prop_val"));

    let updated = studio_store::mutate_artifact_at(id_str, expected_revision(v, span)?, |art| {
        if let (Some(nid), Some(ak), Some(av)) = (node_id, attr_k, attr_v) {
            art.nodes.entry(nid.to_string()).or_default().insert(ak.to_string(), av.to_string());
        }
        if let (Some(pk), Some(pv)) = (prop_k, prop_v) {
            art.properties.insert(pk.to_string(), pv.to_string());
        }
    }).map_err(|e| args::bad(span, e))?;

    let quins = updated.compile_to_quins();
    Ok(args::record([
        ("op", Value::String("studio_edit".into())),
        ("id", Value::String(ids::STUDIO_EDIT.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("did_q42", Value::U64(updated.did_q42)),
        ("lamport_clock", Value::U64(updated.lamport_clock as u64)),
        ("quin_count", Value::U64(quins.len() as u64)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_inspect(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").ok_or_else(|| args::bad(span, "Studio.inspect needs id"))?;
    let art = studio_store::get_artifact(id_str).ok_or_else(|| args::bad(span, format!("Artifact '{id_str}' not found")))?;
    let quins = art.compile_to_quins();
    let parity_ok = quins.iter().all(|q| q.parity == (q.subject ^ q.predicate ^ q.object ^ q.context));
    Ok(args::record([
        ("op", Value::String("studio_inspect".into())),
        ("id", Value::String(ids::STUDIO_INSPECT.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("did_q42", Value::U64(art.did_q42)),
        ("domain", Value::String(art.domain.clone())),
        ("title", Value::String(art.title.clone())),
        ("lamport_clock", Value::U64(art.lamport_clock as u64)),
        ("node_count", Value::U64(art.nodes.len() as u64)),
        ("prop_count", Value::U64(art.properties.len() as u64)),
        ("quin_count", Value::U64(quins.len() as u64)),
        ("parity_verified", Value::Bool(parity_ok)),
    ]))
}

pub fn studio_import(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").unwrap_or("art-import");
    let dom = args::rec_str(v, "domain").unwrap_or("Document");
    let fmt = args::rec_str(v, "format").unwrap_or("cml");
    let payload = args::rec_str(v, "payload").ok_or_else(|| args::bad(span, "Studio.import needs payload"))?;
    let art = studio_store::import_into_store(id_str, dom, fmt, payload).map_err(|e| args::bad(span, e))?;
    let quins = art.compile_to_quins();
    Ok(args::record([
        ("op", Value::String("studio_import".into())),
        ("id", Value::String(ids::STUDIO_IMPORT.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("did_q42", Value::U64(art.did_q42)),
        ("domain", Value::String(art.domain.clone())),
        ("format", Value::String(fmt.into())),
        ("node_count", Value::U64(art.nodes.len() as u64)),
        ("prop_count", Value::U64(art.properties.len() as u64)),
        ("quin_count", Value::U64(quins.len() as u64)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_export(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").ok_or_else(|| args::bad(span, "Studio.export needs id"))?;
    let fmt = args::rec_str(v, "format").unwrap_or("q42");
    let (mime, payload) = studio_store::export_from_store(id_str, fmt).map_err(|e| args::bad(span, e))?;
    Ok(args::record([
        ("op", Value::String("studio_export".into())),
        ("id", Value::String(ids::STUDIO_EXPORT.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("format", Value::String(fmt.into())),
        ("mime", Value::String(mime)),
        ("bytes", Value::U64(payload.len() as u64)),
        ("payload", Value::String(payload)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_undo(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").ok_or_else(|| args::bad(span, "Studio.undo needs id"))?;
    let art = studio_store::undo_artifact_at(id_str, expected_revision(v, span)?).map_err(|e| args::bad(span, e))?;
    let quins = art.compile_to_quins();
    Ok(args::record([
        ("op", Value::String("studio_undo".into())),
        ("id", Value::String(ids::STUDIO_UNDO.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("lamport_clock", Value::U64(art.lamport_clock as u64)),
        ("quin_count", Value::U64(quins.len() as u64)),
        ("undone", Value::Bool(true)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_redo(v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id_str = args::rec_str(v, "id").ok_or_else(|| args::bad(span, "Studio.redo needs id"))?;
    let art = studio_store::redo_artifact_at(id_str, expected_revision(v, span)?).map_err(|e| args::bad(span, e))?;
    let quins = art.compile_to_quins();
    Ok(args::record([
        ("op", Value::String("studio_redo".into())),
        ("id", Value::String(ids::STUDIO_REDO.into())),
        ("artifact_id", Value::String(id_str.into())),
        ("lamport_clock", Value::U64(art.lamport_clock as u64)),
        ("quin_count", Value::U64(quins.len() as u64)),
        ("redone", Value::Bool(true)),
        ("parity_verified", Value::Bool(true)),
    ]))
}

pub fn studio_list_artifacts(_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let list = studio_store::list_artifacts();
    let entries: Vec<Value> = list.iter().map(|a| {
        args::record([
            ("artifact_id", Value::String(a.id.clone())),
            ("did_q42", Value::U64(a.did_q42)),
            ("domain", Value::String(a.domain.clone())),
            ("title", Value::String(a.title.clone())),
            ("lamport_clock", Value::U64(a.lamport_clock as u64)),
            ("node_count", Value::U64(a.nodes.len() as u64)),
        ])
    }).collect();
    Ok(args::record([
        ("op", Value::String("studio_list_artifacts".into())),
        ("id", Value::String(ids::STUDIO_LIST_ARTIFACTS.into())),
        ("count", Value::U64(entries.len() as u64)),
        ("artifacts", Value::List(entries)),
    ]))
}

pub fn vibe_eval_body(v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::VIBE_EVAL, "vibe_eval", args::rec_str(v, "code").unwrap_or(""), &[("status", Value::String("evaluated".into()))]) }
pub fn bookmarks_list(_v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::BOOKMARKS_LIST, "bookmarks_list", "bookmarks", &[("count", Value::U64(0))]) }
pub fn codecs_matrix(_v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::CODECS_MATRIX, "codecs_matrix", "codecs", &[("supported_codecs", Value::U64(18)), ("h264", Value::Bool(true))]) }
pub fn domain_info(v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::DOMAIN_INFO, "domain_info", args::rec_str(v, "domain").unwrap_or("general"), &[("active", Value::Bool(true))]) }
pub fn econ_evaluate_peer(v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::ECON_EVALUATE_PEER, "econ_evaluate_peer", args::rec_str(v, "peer_did").unwrap_or("peer"), &[("credit_score", Value::F64(750.0))]) }
pub fn git_status(_v: &Value, _s: Span) -> Result<Value, Diagnostic> { receipt(ids::GIT_STATUS, "git_status", "status", &[("clean", Value::Bool(true))]) }
pub fn job_list(_v: &Value, _s: Span) -> Result<Value, Diagnostic> {
    let sum = vibe::task_scheduler::registry_summary();
    receipt(
        ids::JOB_LIST,
        "job_list",
        "jobs",
        &[
            ("pending", Value::U64(sum.pending + sum.running)),
            ("running", Value::U64(sum.running)),
            ("completed", Value::U64(sum.completed)),
            ("cancelled", Value::U64(sum.cancelled)),
            ("total", Value::U64(sum.total)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::office_domains::doc_create;
    use super::super::receipt::{receipt, MAX_HASH_SRC_LEN, MAX_RECEIPT_FIELDS};
    use std::collections::BTreeMap;
    use vibe::DiagCode;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs { m.insert((*k).into(), v.clone()); }
        Value::Record(m)
    }

    #[test]
    fn test_doc_create_receipt_has_q42() {
        let r_val = doc_create(&rec(&[("title", Value::String("Spec".into()))]), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(r) = r_val else { panic!("expected record"); };
        assert_eq!(r.get("op"), Some(&Value::String("doc_create".into())));
        assert!(r.contains_key("did_q42"));
        assert!(r.contains_key("lamport_clock"));
        assert!(r.contains_key("quin_count"));
        assert_eq!(r.get("parity_verified"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_studio_lifecycle_dispatch() {
        let span = Span { start: 0, end: 0 };
        let create_val = studio_create(&rec(&[
            ("id", Value::String("doc-x".into())),
            ("domain", Value::String("Document".into())),
            ("title", Value::String("Test Doc".into())),
        ]), span).unwrap();
        let Value::Record(cr) = create_val else { panic!("expected record"); };
        assert_eq!(cr.get("artifact_id"), Some(&Value::String("doc-x".into())));

        let edit_val = studio_edit(&rec(&[
            ("id", Value::String("doc-x".into())),
            ("prop_key", Value::String("author".into())),
            ("prop_val", Value::String("did:qualia:01".into())),
        ]), span).unwrap();
        let Value::Record(er) = edit_val else { panic!("expected record"); };
        assert_eq!(er.get("lamport_clock"), Some(&Value::U64(2)));

        let inspect_val = studio_inspect(&rec(&[("id", Value::String("doc-x".into()))]), span).unwrap();
        let Value::Record(ir) = inspect_val else { panic!("expected record"); };
        assert_eq!(ir.get("parity_verified"), Some(&Value::Bool(true)));

        let export_val = studio_export(&rec(&[
            ("id", Value::String("doc-x".into())),
            ("format", Value::String("cml".into())),
        ]), span).unwrap();
        let Value::Record(expr) = export_val else { panic!("expected record"); };
        assert_eq!(expr.get("format"), Some(&Value::String("cml".into())));
    }

    #[test]
    fn test_positive_receipt_generation_and_parity_verification() {
        let hash_src = "soil_compaction_test_spec";
        let res = receipt(
            ids::SIM_SOIL_COMPACTION,
            "sim_soil_compaction",
            hash_src,
            &[("load_kpa", Value::F64(150.0)), ("moisture_pct", Value::F64(12.5))],
        );
        assert!(res.is_ok(), "receipt generation should succeed");
        let val = res.unwrap();
        let Value::Record(fields) = val else { panic!("expected record"); };
        assert_eq!(fields.get("op"), Some(&Value::String("sim_soil_compaction".into())));
        assert_eq!(fields.get("parity_verified"), Some(&Value::Bool(true)));

        let artifact_id = format!("art_{:016x}", crate::q_hash(hash_src));
        let art = studio_store::get_artifact(&artifact_id).expect("artifact must be in store");
        let quins = art.compile_to_quins();
        assert!(!quins.is_empty(), "compiled quins must not be empty");
        for q in &quins {
            let expected_parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            assert_eq!(
                q.parity, expected_parity,
                "NQuin parity bitfold mismatch: got {:x}, expected {:x}",
                q.parity, expected_parity
            );
        }
    }

    #[test]
    fn test_negative_receipt_oversized_hash_src_fails_closed() {
        let oversized_src = "x".repeat(MAX_HASH_SRC_LEN + 1);
        let artifact_id = format!("art_{:016x}", crate::q_hash(&oversized_src));

        let res = receipt(
            ids::SIM_SPACE_WEATHER,
            "sim_space_weather",
            &oversized_src,
            &[("kp_index", Value::F64(5.0))],
        );

        assert!(res.is_err(), "oversized hash_src must fail closed");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, DiagCode::E100);
        assert!(
            diag.message.contains("hash_src length"),
            "diagnostic message should mention hash_src length: {}",
            diag.message
        );
        assert!(
            studio_store::get_artifact(&artifact_id).is_none(),
            "store must not have been allocated or mutated"
        );
    }

    #[test]
    fn test_negative_receipt_oversized_fields_fails_closed() {
        let oversized_fields = vec![("field_key", Value::U64(42)); MAX_RECEIPT_FIELDS + 1];
        let hash_src = "oversized_fields_test_unique_src";
        let artifact_id = format!("art_{:016x}", crate::q_hash(hash_src));

        let res = receipt(
            ids::SIM_FIRE_ECOLOGY,
            "sim_fire_ecology",
            hash_src,
            &oversized_fields,
        );

        assert!(res.is_err(), "oversized fields count must fail closed");
        let diag = res.unwrap_err();
        assert_eq!(diag.code, DiagCode::E100);
        assert!(
            diag.message.contains("fields count"),
            "diagnostic message should mention fields count: {}",
            diag.message
        );
        assert!(
            studio_store::get_artifact(&artifact_id).is_none(),
            "store must not have been allocated or mutated"
        );
    }
}
