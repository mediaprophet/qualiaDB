//! POET Host Semantic Instruments Engine Bridge.
//!
//! Connects semantic-instrument collectables, manufacture gates, catalogue discovery,
//! generic runner execution, and RPL capability gap evaluation into POET Shed invoke.
//! Follows AGENTS.md §0-B (< 360 lines).

use super::args;
use crate::semantic_instruments::{
    assert_can_publish, build_collectable, demo_badge_10d, demo_catalog, evaluate_rpl,
    hcf_from_manifest, inspect_for_publish, open_collectable, preflight, reference_catalog,
    run_entry, CollectableParts, ContentCategory, EvidenceBasis, EvidenceItem,
    InstalledRelease, InstrumentManifest, LocalCatalogue, LocalRegistry, RunOutcome, RunRequest,
    COLLECTABLE_FORMAT_VERSION,
};
use vibe::{Diagnostic, Span, Value};

fn resolve_bytes(slug_or_id: &str, direct: Option<Vec<u8>>) -> Option<(String, Vec<u8>)> {
    if let Some(b) = direct { if !b.is_empty() { return Some((slug_or_id.to_string(), b)); } }
    for seed in demo_catalog() {
        if seed.slug == slug_or_id || seed.release_id == slug_or_id || seed.instrument_id == slug_or_id {
            if let Ok(b) = crate::semantic_instruments::build_demo(seed.slug) { return Some((seed.release_id.to_string(), b)); }
        }
    }
    for seed in reference_catalog() {
        if seed.slug == slug_or_id || seed.release_id == slug_or_id || seed.instrument_id == slug_or_id {
            if let Ok(b) = crate::semantic_instruments::build_reference(seed.slug) { return Some((seed.release_id.to_string(), b)); }
        }
    }
    let art_id = if slug_or_id.starts_with("si:") { slug_or_id.to_string() } else { format!("si:{slug_or_id}") };
    let art = super::studio_store::get_artifact(&art_id)?;
    let slug = art.properties.get("slug").cloned().unwrap_or_else(|| slug_or_id.to_string());
    let rel_id = art.properties.get("release_id").cloned().unwrap_or_else(|| format!("https://ns.webizen.org/demo/{slug}/releases/1.0.0"));
    let inst_id = art.properties.get("instrument_id").cloned().unwrap_or_else(|| format!("https://ns.webizen.org/demo/{slug}"));
    let entry_point = art.properties.get("entry_point").cloned().unwrap_or_else(|| "assess".into());
    let def_n3 = art.properties.get("definition_n3").cloned().unwrap_or_default();
    let cat = if art.properties.get("category").map(|s| s.as_str()) == Some("reference") { ContentCategory::Reference } else { ContentCategory::Demo };
    let manifest = InstrumentManifest {
        format_version: COLLECTABLE_FORMAT_VERSION.into(), instrument_id: inst_id, release_id: rel_id.clone(),
        version: "1.0.0".into(), name: art.title.clone(), purpose: "demonstration-and-development".into(),
        content_category: cat, prohibited_interpretation: "operational-clinical-diagnosis".into(),
        domain: "https://ns.webizen.org/demo/concepts/custom".into(), ontology_reference: "https://ns.webizen.org/cml/v1".into(),
        entry_point, citation: "QualiaDB SI Reference".into(), honesty_notice: "Demo seed notice: labelled for demonstration only.".into(),
        incomplete_input: "held".into(), result_kind: "demo-claim".into(), licence: "CC-BY-4.0".into(), authored_by: "did:q42:author".into(),
        accessible_text: "10D Badge for Custom Semantic Instrument".into(), visual_media_type: crate::semantic_instruments::visual::VISUAL_MEDIA_TYPE.into(),
        content_digest: String::new(), extra: Default::default(),
    };
    let hcf = hcf_from_manifest(&manifest);
    let parts = CollectableParts { manifest, definition_n3: def_n3.into_bytes(), hcf, visual_10d: demo_badge_10d().to_vec(), small_q42: None };
    build_collectable(parts).ok().map(|b| (rel_id, b))
}

fn instrument_catalog(_args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mut items = Vec::new();
    for s in demo_catalog() {
        items.push(args::record([("slug", Value::String(s.slug.into())), ("name", Value::String(s.name.into())), ("release_id", Value::String(s.release_id.into())), ("category", Value::String("demo".into())), ("entry_point", Value::String(s.entry_point.into())), ("domain", Value::String(s.domain.into()))]));
    }
    for s in reference_catalog() {
        items.push(args::record([("slug", Value::String(s.slug.into())), ("name", Value::String(s.name.into())), ("release_id", Value::String(s.release_id.into())), ("category", Value::String("reference".into())), ("entry_point", Value::String(s.entry_point.into())), ("domain", Value::String(s.domain.into()))]));
    }
    let (mut user_count, store_arts) = (0u64, super::studio_store::list_artifacts());
    for art in store_arts {
        if art.domain == "SemanticInstrument" || art.id.starts_with("si:") {
            user_count += 1;
            let slug = art.properties.get("slug").cloned().unwrap_or_else(|| art.id.clone());
            let rel_id = art.properties.get("release_id").cloned().unwrap_or_else(|| art.id.clone());
            let entry = art.properties.get("entry_point").cloned().unwrap_or_else(|| "assess".into());
            let cat = art.properties.get("category").cloned().unwrap_or_else(|| "demo".into());
            let dom = art.properties.get("domain").cloned().unwrap_or_else(|| "custom".into());
            items.push(args::record([("slug", Value::String(slug)), ("name", Value::String(art.title)), ("release_id", Value::String(rel_id)), ("category", Value::String(cat)), ("entry_point", Value::String(entry)), ("domain", Value::String(dom))]));
        }
    }
    let (d_len, r_len) = (demo_catalog().len() as u64, reference_catalog().len() as u64);
    Ok(args::record([
        ("status", Value::String("ok".into())), ("op", Value::String("instrument_catalog".into())),
        ("demo_count", Value::U64(d_len)), ("reference_count", Value::U64(r_len)),
        ("user_count", Value::U64(user_count)), ("total_count", Value::U64(d_len + r_len + user_count)),
        ("instruments", Value::List(items)), ("shell", Value::String("poet".into())),
    ]))
}

fn instrument_define(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (slug, name) = (args::rec_str(args_v, "slug").unwrap_or("custom-inst"), args::rec_str(args_v, "name").unwrap_or("Custom Semantic Instrument"));
    let (entry_point, def_n3) = (args::rec_str(args_v, "entry_point").unwrap_or("assess"), args::rec_str(args_v, "definition_n3").unwrap_or("@prefix si: <https://ns.webizen.org/semantic-instrument/> .\n"));

    if slug.contains("Host.") || name.contains("Host.") || entry_point.contains("Host.") {
        return Err(args::bad(span, "unknown entry point (not a Host ID)"));
    }
    if entry_point != "assess" && entry_point != "recognise" {
        return Err(args::bad(span, "held / not yet — entry point must be assess or recognise"));
    }
    if def_n3.to_ascii_lowercase().contains("owl:thing") {
        return Err(args::bad(span, "Living-safe: a NaturalPerson is never owl:Thing; persons are rdfs:Class, not the OWL universal superclass."));
    }

    let (version, purpose) = (args::rec_str(args_v, "version").unwrap_or("1.0.0"), args::rec_str(args_v, "purpose").unwrap_or("demonstration-and-development"));
    let (domain, category) = (args::rec_str(args_v, "domain").unwrap_or("https://ns.webizen.org/demo/concepts/custom"), args::rec_str(args_v, "category").unwrap_or("demo"));
    let prohibited = args::rec_str(args_v, "prohibited_interpretation").unwrap_or("operational-clinical-diagnosis");
    let ontology = args::rec_str(args_v, "ontology_reference").unwrap_or("https://ns.webizen.org/cml/v1");
    let citation = args::rec_str(args_v, "citation").unwrap_or("QualiaDB SI Reference");
    let honesty = args::rec_str(args_v, "honesty_notice").unwrap_or("Demo seed notice: labelled for demonstration only.");
    let (incomplete, result_kind) = (args::rec_str(args_v, "incomplete_input").unwrap_or("held"), args::rec_str(args_v, "result_kind").unwrap_or("demo-claim"));
    let (licence, author) = (args::rec_str(args_v, "licence").unwrap_or("CC-BY-4.0"), args::rec_str(args_v, "authored_by").unwrap_or("did:q42:author"));
    let a11y = args::rec_str(args_v, "accessible_text").unwrap_or("10D Badge for Custom Semantic Instrument");

    let inst_id = args::rec_str(args_v, "instrument_id").map(|s| s.to_string()).unwrap_or_else(|| format!("https://ns.webizen.org/demo/{slug}"));
    let rel_id = args::rec_str(args_v, "release_id").map(|s| s.to_string()).unwrap_or_else(|| format!("https://ns.webizen.org/demo/{slug}/releases/{version}"));
    let cat = match category.to_ascii_lowercase().as_str() { "reference" => ContentCategory::Reference, "operational" => ContentCategory::Operational, _ => ContentCategory::Demo };

    let manifest = InstrumentManifest {
        format_version: COLLECTABLE_FORMAT_VERSION.into(), instrument_id: inst_id.clone(), release_id: rel_id.clone(),
        version: version.into(), name: name.into(), purpose: purpose.into(), content_category: cat, prohibited_interpretation: prohibited.into(),
        domain: domain.into(), ontology_reference: ontology.into(), entry_point: entry_point.into(), citation: citation.into(),
        honesty_notice: honesty.into(), incomplete_input: incomplete.into(), result_kind: result_kind.into(), licence: licence.into(),
        authored_by: author.into(), accessible_text: a11y.into(), visual_media_type: crate::semantic_instruments::visual::VISUAL_MEDIA_TYPE.into(),
        content_digest: String::new(), extra: Default::default(),
    };

    let hcf = hcf_from_manifest(&manifest);
    let parts = CollectableParts { manifest, definition_n3: def_n3.as_bytes().to_vec(), hcf, visual_10d: demo_badge_10d().to_vec(), small_q42: None };
    let hmc_bytes = build_collectable(parts).map_err(|e| args::bad(span, format!("build: {e}")))?;
    let inspection = inspect_for_publish(&hmc_bytes).map_err(|e| args::bad(span, format!("inspect: {e}")))?;
    assert_can_publish(&inspection).map_err(|e| args::bad(span, format!("publish gate: {e}")))?;

    let mut cat_db = LocalCatalogue::new();
    let listing = cat_db.publish(hmc_bytes.clone()).map_err(|e| args::bad(span, format!("publish: {e}")))?;

    let art_id = format!("si:{slug}");
    let mut art = super::studio_store::StudioArtifact::new(&art_id, "SemanticInstrument", name);
    for (k, v) in &[("slug", slug), ("release_id", &rel_id), ("instrument_id", &inst_id), ("entry_point", entry_point), ("content_digest", &listing.content_digest), ("category", category), ("domain", domain), ("definition_n3", def_n3)] {
        art.properties.insert((*k).into(), (*v).into());
    }
    art.properties.insert("hmc_size".into(), hmc_bytes.len().to_string());
    super::studio_store::upsert_artifact(art);

    Ok(args::record([
        ("status", Value::String("defined".into())), ("op", Value::String("instrument_define".into())),
        ("slug", Value::String(slug.into())), ("instrument_id", Value::String(inst_id)),
        ("release_id", Value::String(rel_id)), ("entry_point", Value::String(entry_point.into())),
        ("content_digest", Value::String(listing.content_digest)), ("byte_length", Value::U64(hmc_bytes.len() as u64)),
        ("published", Value::Bool(true)), ("shell", Value::String("poet".into())),
    ]))
}

fn instrument_inspect(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let slug = args::rec_str(args_v, "slug").or_else(|| args::rec_str(args_v, "release_id")).unwrap_or("unit-convert");
    let direct = args::rec_u8_list(args_v, "bytes");
    let (_rel_id, bytes) = resolve_bytes(slug, direct).ok_or_else(|| args::bad(span, format!("collectable not found: {slug}")))?;
    let opened = open_collectable(&bytes).map_err(|e| args::bad(span, format!("open: {e}")))?;
    let inspection = inspect_for_publish(&bytes).map_err(|e| args::bad(span, format!("inspect: {e}")))?;
    let cat_str = format!("{:?}", opened.manifest.content_category).to_ascii_lowercase();
    Ok(args::record([
        ("status", Value::String("inspected".into())), ("op", Value::String("instrument_inspect".into())),
        ("release_id", Value::String(opened.manifest.release_id)), ("instrument_id", Value::String(opened.manifest.instrument_id)),
        ("name", Value::String(opened.manifest.name)), ("entry_point", Value::String(opened.manifest.entry_point)),
        ("category", Value::String(cat_str)), ("labelled", Value::Bool(inspection.labelled)),
        ("has_source_n3", Value::Bool(inspection.has_source_n3)), ("has_graph_q42", Value::Bool(inspection.has_graph_q42)),
        ("has_visual_10d", Value::Bool(inspection.has_visual_10d)), ("round_trip_loss", Value::Bool(inspection.round_trip_loss)),
        ("bundle_digest", Value::String(opened.bundle_digest)), ("content_digest", Value::String(opened.manifest.content_digest)),
        ("shell", Value::String("poet".into())),
    ]))
}

fn install_helper(args_v: &Value, span: Span, closed: bool) -> Result<(String, InstalledRelease), Diagnostic> {
    let slug = args::rec_str(args_v, "slug").or_else(|| args::rec_str(args_v, "release_id")).unwrap_or("unit-convert");
    let (rel_id, bytes) = resolve_bytes(slug, args::rec_u8_list(args_v, "bytes")).ok_or_else(|| args::bad(span, format!("collectable not found: {slug}")))?;
    let mut catalogue = LocalCatalogue::new();
    catalogue.publish(bytes).map_err(|e| args::bad(span, format!("publish: {e}")))?;
    let mut registry = LocalRegistry::new();
    catalogue.verify_and_collect(&rel_id, &mut registry, closed).map_err(|e| args::bad(span, format!("collect: {e}")))?;
    let inst = registry.activate(&rel_id).map_err(|e| args::bad(span, format!("activate: {e}")))?;
    Ok((rel_id, inst.clone()))
}

fn instrument_collect(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let closed = args::rec_bool(args_v, "closed").unwrap_or(true);
    let (rel_id, _) = install_helper(args_v, span, closed)?;
    Ok(args::record([("status", Value::String("collected".into())), ("op", Value::String("instrument_collect".into())), ("release_id", Value::String(rel_id)), ("is_installed", Value::Bool(true)), ("closed", Value::Bool(closed)), ("shell", Value::String("poet".into()))]))
}

fn instrument_activate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (rel_id, inst) = install_helper(args_v, span, true)?;
    Ok(args::record([("status", Value::String("activated".into())), ("op", Value::String("instrument_activate".into())), ("release_id", Value::String(rel_id)), ("content_digest", Value::String(inst.content_digest)), ("shell", Value::String("poet".into()))]))
}

fn instrument_run(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let slug = args::rec_str(args_v, "slug").or_else(|| args::rec_str(args_v, "release_id")).unwrap_or("unit-convert");
    let entry_point = args::rec_str(args_v, "entry_point").unwrap_or("assess");
    if slug.contains("Host.") || entry_point.contains("Host.") {
        return Err(args::bad(span, "unknown entry point (not a Host ID)"));
    }
    if entry_point != "assess" && entry_point != "recognise" {
        return Err(args::bad(span, "held / not yet — entry point must be assess or recognise"));
    }
    let input_bytes = if let Some(raw) = args::rec_u8_list(args_v, "input") {
        raw
    } else if let Some(s) = args::rec_str(args_v, "input") {
        s.as_bytes().to_vec()
    } else {
        b"sample-payload".to_vec()
    };
    let now = args::rec_u64(args_v, "now").unwrap_or(0);
    let max_steps = args::rec_u64(args_v, "max_steps").unwrap_or(1000);
    let cancelled = args::rec_bool(args_v, "cancelled").unwrap_or(false);
    let parent_receipt = args::rec_str(args_v, "parent_receipt");

    let direct = args::rec_u8_list(args_v, "bytes");
    let (rel_id, bytes) = resolve_bytes(slug, direct).ok_or_else(|| args::bad(span, format!("collectable not found: {slug}")))?;
    let mut catalogue = LocalCatalogue::new();
    catalogue.publish(bytes).map_err(|e| args::bad(span, format!("publish: {e}")))?;
    let mut registry = LocalRegistry::new();
    catalogue.verify_and_collect(&rel_id, &mut registry, true).map_err(|e| args::bad(span, format!("collect: {e}")))?;

    let req = RunRequest {
        release_id: &rel_id, entry_point, input: &input_bytes, now: now as u32,
        cancelled, max_input_bytes: 1024 * 1024, max_steps: max_steps as u32,
        parent_receipt: parent_receipt.map(|s| s.to_string()),
    };
    let installed = preflight(&registry, &req).map_err(|e| args::bad(span, format!("preflight: {e}")))?;
    let outcome = run_entry(&registry, installed, &req);
    let (status, receipt) = match outcome {
        RunOutcome::Completed { receipt } => ("completed", receipt),
        RunOutcome::Held { receipt } => ("held", receipt),
        RunOutcome::Refused { error } => return Err(args::bad(span, format!("refused: {error}"))),
    };
    Ok(args::record([
        ("status", Value::String(status.into())), ("op", Value::String("instrument_run".into())),
        ("receipt_id", Value::String(receipt.id())), ("release_id", Value::String(receipt.release_id)),
        ("entry_point", Value::String(receipt.entry_point)), ("outcome", Value::String(receipt.outcome)),
        ("runner", Value::String(receipt.runner)), ("input_commitment", Value::String(receipt.input_commitment)),
        ("output_commitment", Value::String(receipt.output_commitment)), ("shell", Value::String("poet".into())),
    ]))
}

fn instrument_rpl_eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let required = args::rec_str_list(args_v, "required").unwrap_or_default();
    let mut evidence_items = Vec::new();
    if let Some(list_val) = args::rec(args_v, "evidence").and_then(args::list) {
        for val in list_val {
            if let Some(cap) = args::as_str(val) {
                evidence_items.push(EvidenceItem { capability: cap.to_string(), basis: EvidenceBasis::Formal, available: true, private: false });
            } else if let Value::Record(map) = val {
                let cap = map.get("capability").and_then(args::as_str).unwrap_or("");
                let basis_str = map.get("basis").and_then(args::as_str).unwrap_or("formal");
                let basis = match basis_str.to_ascii_lowercase().as_str() {
                    "prior_learning" | "priorlearning" => EvidenceBasis::PriorLearning,
                    "experiential" => EvidenceBasis::Experiential,
                    "peer_attested" | "peerattested" => EvidenceBasis::PeerAttested,
                    _ => EvidenceBasis::Formal,
                };
                let avail = map.get("available").and_then(args::as_bool).unwrap_or(true);
                let priv_flag = map.get("private").and_then(args::as_bool).unwrap_or(false);
                if !cap.is_empty() {
                    evidence_items.push(EvidenceItem { capability: cap.to_string(), basis, available: avail, private: priv_flag });
                }
            }
        }
    }
    let parse_pairs = |k1: &str, k2: &str| -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Some(xs) = args::rec(args_v, k1).or_else(|| args::rec(args_v, k2)).and_then(args::list) {
            for v in xs {
                if let Some(p) = args::list(v) {
                    if p.len() >= 2 {
                        if let (Some(a), Some(b)) = (args::as_str(&p[0]), args::as_str(&p[1])) { out.push((a.to_string(), b.to_string())); }
                    }
                }
            }
        }
        out
    };
    let auth_pairs = parse_pairs("authorised_equivalences", "authorized_equivalences");
    let unauth_pairs = parse_pairs("unauthorised_equivalences", "unauthorized_equivalences");
    let goal = args::rec_str(args_v, "goal");
    let mut scratch = [0u64; 16];
    let res = evaluate_rpl(&required, &evidence_items, &auth_pairs, &unauth_pairs, &[], goal, &mut scratch)
        .map_err(|e| args::bad(span, format!("rpl: {e}")))?;

    let sat: Vec<Value> = res.satisfied.into_iter().map(Value::String).collect();
    let eq: Vec<Value> = res.equivalent.into_iter().map(Value::String).collect();
    let unres: Vec<Value> = res.unresolved.into_iter().map(Value::String).collect();
    let unmet: Vec<Value> = res.unmet.into_iter().map(Value::String).collect();
    let is_met = unmet.is_empty() && unres.is_empty();

    Ok(args::record([
        ("status", Value::String("ok".into())), ("op", Value::String("instrument_rpl_eval".into())),
        ("satisfied", Value::List(sat)), ("equivalent", Value::List(eq)),
        ("unresolved", Value::List(unres)), ("unmet", Value::List(unmet)),
        ("learning_path_cost", res.learning_path_cost.map(|c| Value::U64(c as u64)).unwrap_or(Value::Null)),
        ("is_met", Value::Bool(is_met)), ("shell", Value::String("poet".into())),
    ]))
}

/// Dispatches a Semantic Instrument operation by op name.
pub fn handle_semantic_instrument(op: &str, args_v: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match op {
        "instrument_catalog" | "si_catalog" => Some(instrument_catalog(args_v, span)),
        "instrument_define" | "si_define" => Some(instrument_define(args_v, span)),
        "instrument_inspect" | "si_inspect" => Some(instrument_inspect(args_v, span)),
        "instrument_collect" | "si_collect" => Some(instrument_collect(args_v, span)),
        "instrument_activate" | "si_activate" => Some(instrument_activate(args_v, span)),
        "instrument_run" | "si_run" => Some(instrument_run(args_v, span)),
        "instrument_rpl_eval" | "si_rpl_eval" => Some(instrument_rpl_eval(args_v, span)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn rec(p: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in p { m.insert((*k).into(), v.clone()); }
        Value::Record(m)
    }

    #[test]
    fn test_catalog_define_inspect() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(c) = instrument_catalog(&rec(&[]), span).unwrap() else { panic!("rec"); };
        assert_eq!(c.get("status"), Some(&Value::String("ok".into())));
        assert_eq!(c.get("demo_count"), Some(&Value::U64(6)));
        assert_eq!(c.get("reference_count"), Some(&Value::U64(3)));

        let def_val = rec(&[("slug", Value::String("unit-calc".into())), ("name", Value::String("Unit Calculator".into())), ("entry_point", Value::String("assess".into())), ("definition_n3", Value::String("@prefix si: <https://ns.webizen.org/semantic-instrument/> .\n".into()))]);
        let Value::Record(d) = instrument_define(&def_val, span).unwrap() else { panic!("rec"); };
        assert_eq!(d.get("status"), Some(&Value::String("defined".into())));

        let insp_val = rec(&[("slug", Value::String("unit-calc".into()))]);
        let Value::Record(i) = instrument_inspect(&insp_val, span).unwrap() else { panic!("rec"); };
        assert_eq!(i.get("status"), Some(&Value::String("inspected".into())));
    }

    #[test]
    fn test_run_rpl_guards() {
        let span = Span { start: 0, end: 0 };
        let run_val = rec(&[("slug", Value::String("unit-convert".into())), ("entry_point", Value::String("assess".into())), ("input", Value::String("42 km/h".into()))]);
        let Value::Record(r) = instrument_run(&run_val, span).unwrap() else { panic!("rec"); };
        assert_eq!(r.get("status"), Some(&Value::String("completed".into())));

        let rpl_val = rec(&[("required", Value::List(vec![Value::String("https://ns.webizen.org/cap/math".into())])), ("evidence", Value::List(vec![Value::String("https://ns.webizen.org/cap/math".into())]))]);
        let Value::Record(p) = instrument_rpl_eval(&rpl_val, span).unwrap() else { panic!("rec"); };
        assert_eq!(p.get("is_met"), Some(&Value::Bool(true)));

        assert!(instrument_define(&rec(&[("slug", Value::String("Host.evil".into())), ("entry_point", Value::String("assess".into()))]), span).is_err());
        assert!(instrument_define(&rec(&[("slug", Value::String("safe-slug".into())), ("entry_point", Value::String("kernel_exec".into()))]), span).is_err());
        assert!(instrument_define(&rec(&[("slug", Value::String("safe-slug".into())), ("entry_point", Value::String("assess".into())), ("definition_n3", Value::String("ex:Person a owl:Thing .\n".into()))]), span).is_err());
    }
}
